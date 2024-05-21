use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};
use spl_token::instruction::transfer as spl_transfer;
use std::str::FromStr;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct ConfigData {
    /// nest presale total amount
    pub nest_total: u64,
    /// nest presaled nest amount
    pub presale_total: u64,
}

impl ConfigData {
    pub const ACCOUNT_SPACE: usize = 8 + 8;
    pub const SEED_CONFIG: &'static str = "CONFIG";
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct PresaleData {
    /// presale nest total amount
    pub nest_total: u64,
    /// claimed nest amount
    pub nest_claim: u64,
    /// cliff nest amount
    pub nest_cliff: u64,
    /// seed bump
    pub bump: u8,
}

impl PresaleData {
    pub const ACCOUNT_SPACE: usize = 8 + 8 + 8 + 8 + 1;
    pub const SEED_PRESALE: &'static str = "PRESALE";
    pub const SEED_BANK: &'static str = "BANK";
}

const RATIO_BOOST: u64 = 10000;
const RATIO: u64 = 5; // 1e8/(20*1e6)
const CLIFF_PERCENT: u64 = 1000; // 10%
const UNLOCK_PERCENT: u64 = 750; // 7.5%

const PER_MONTH_SECOND = 60 * 60 * 24 * 30;
const PERIOD_TOTAL: u64 = 12; // 12month
const PRESALE_ENDTIME: i64 = 1719590399; // 2024-06-28 23:59:59
const UNLOCK_START_TIME: i64 = 1722182399; // 2024-07-28 23:59:59
const NEST_TOTAL_AMOUNT: u64 = 15000000_00000000; // 1500W
const MIN_USDT_AMOUNT: u64 = 10_000000; // 10USDT

// presale usdt receiver token address
const USDT_RECEIVER_ADDRESS: &str = "DqtF****";
const ADMIN_ADDRESS: &str = "4nnb****";

entrypoint!(process_instruction);

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum TransferInstruction {
    CreateBank,
    CreateConfig,
    CreatePresale,
    Presale(u64),
    Claim,
    Withdraw(u64),
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = TransferInstruction::try_from_slice(instruction_data)?;
    match instruction {
        TransferInstruction::CreateBank => create_bank(program_id, accounts),
        TransferInstruction::CreateConfig => create_config(program_id, accounts),
        TransferInstruction::CreatePresale => create_presale(program_id, accounts),
        TransferInstruction::Presale(amount) => presale(program_id, accounts, amount),
        TransferInstruction::Claim => claim(program_id, accounts),
        TransferInstruction::Withdraw(amount) => withdraw(program_id, accounts, amount),
    }
}

// create bank_pda account
// nest token transfer from bank_pda account
pub fn create_bank(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let owner_account = next_account_info(accounts_iter)?; // payer owner account
    let bank_pda: &AccountInfo = next_account_info(accounts_iter)?; // bank pda account
    let system_program = next_account_info(accounts_iter)?;

    // check admin
    let admin_account = Pubkey::from_str(ADMIN_ADDRESS).unwrap();
    if owner_account.key.ne(&admin_account) {
        return Err(ProgramError::InvalidAccountData);
    }

    let (pda, bump) =
        Pubkey::find_program_address(&[PresaleData::SEED_BANK.as_bytes()], program_id);
    msg!("pda account {:?} {}", pda, bump);
    if bank_pda.key.ne(&pda) {
        msg!("owner bank_pda account mismatch");
        return Err(ProgramError::InvalidAccountData);
    }

    let account_span = 0usize;
    let lamports_required = (Rent::get()?).minimum_balance(account_span);

    invoke_signed(
        &system_instruction::create_account(
            owner_account.key,
            &pda,
            lamports_required,
            account_span as u64,
            &system_program.key,
        ),
        &[
            owner_account.clone(),
            bank_pda.clone(),
            system_program.clone(),
        ],
        &[&[PresaleData::SEED_BANK.as_bytes(), &[bump]]],
    )?;

    Ok(())
}

// create config_pda account
// nest_total presale_total save
pub fn create_config(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let owner_account = next_account_info(accounts_iter)?; // payer owner account
    let config_pda: &AccountInfo = next_account_info(accounts_iter)?; // config pda account
    let system_program = next_account_info(accounts_iter)?;

    // check admin
    let admin_account = Pubkey::from_str(ADMIN_ADDRESS).unwrap();
    if owner_account.key.ne(&admin_account) {
        return Err(ProgramError::InvalidAccountData);
    }

    let (pda, bump) =
        Pubkey::find_program_address(&[ConfigData::SEED_CONFIG.as_bytes()], program_id);
    msg!("pda account {:?} {}", pda, bump);
    if config_pda.key.ne(&pda) {
        msg!("owner config_pda account mismatch");
        return Err(ProgramError::InvalidAccountData);
    }

    let mut config_data = ConfigData {
        nest_total: 0,
        presale_total: 0,
    };
    // let account_span = PresaleData::ACCOUNT_SPACE;
    let account_span = (config_data.try_to_vec()?).len();
    msg!(
        "account_span {} {}",
        account_span,
        ConfigData::ACCOUNT_SPACE
    );
    let lamports_required = (Rent::get()?).minimum_balance(account_span);

    invoke_signed(
        &system_instruction::create_account(
            owner_account.key,
            &pda,
            lamports_required,
            account_span as u64,
            program_id,
        ),
        &[
            owner_account.clone(),
            config_pda.clone(),
            system_program.clone(),
        ],
        &[&[ConfigData::SEED_CONFIG.as_bytes(), &[bump]]],
    )?;

    config_data.nest_total = NEST_TOTAL_AMOUNT;
    config_data.serialize(&mut *config_pda.data.borrow_mut())?;
    msg!("config_data {:?}", &config_data);

    Ok(())
}

// create presale_pda data account
// presale_data save presale_pda account
pub fn create_presale(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let owner_account = next_account_info(accounts_iter)?; // payer owner account
    let presale_pda: &AccountInfo = next_account_info(accounts_iter)?; // presale pda account
    let system_program = next_account_info(accounts_iter)?;

    let (pda, bump) = Pubkey::find_program_address(
        &[
            PresaleData::SEED_PRESALE.as_bytes(),
            owner_account.key.as_ref(),
        ],
        program_id,
    );
    msg!("pda account {:?} {}", pda, bump);
    if presale_pda.key.ne(&pda) {
        msg!("owner presale_pda account mismatch");
        return Err(ProgramError::InvalidAccountData);
    }

    let presale_data = PresaleData {
        nest_total: 0,
        nest_claim: 0,
        nest_cliff: 0,
        bump,
    };
    // let account_span = PresaleData::ACCOUNT_SPACE;
    let account_span = (presale_data.try_to_vec()?).len();
    msg!(
        "account_span {} {}",
        account_span,
        PresaleData::ACCOUNT_SPACE
    );
    let lamports_required = (Rent::get()?).minimum_balance(account_span);

    invoke_signed(
        &system_instruction::create_account(
            owner_account.key,
            &pda,
            lamports_required,
            account_span as u64,
            program_id,
        ),
        &[
            owner_account.clone(),
            presale_pda.clone(),
            system_program.clone(),
        ],
        &[&[
            PresaleData::SEED_PRESALE.as_bytes(),
            owner_account.key.as_ref(),
            &[bump],
        ]],
    )?;

    presale_data.serialize(&mut *presale_pda.data.borrow_mut())?;
    msg!("presale_data {:?}", &presale_data);

    Ok(())
}

// user transfer usdt presale buy nest token
// amount - presale usdt amount
pub fn presale(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let token_program = next_account_info(accounts_iter)?; // token program id
    let owner_account = next_account_info(accounts_iter)?; // payer owner account
    let source_ata = next_account_info(accounts_iter)?; // payer owner usdt ata account
    let destination_ata = next_account_info(accounts_iter)?; // usdt receiver ata account
    let presale_pda = next_account_info(accounts_iter)?; // payer owner presale pda account
    let config_pda = next_account_info(accounts_iter)?; // config pda account

    msg!("presale_pda account {:?}", presale_pda.key);

    if !owner_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // check usdt receiver ata account
    let usdt_receiver_pubkey = Pubkey::from_str(USDT_RECEIVER_ADDRESS).unwrap();
    if destination_ata.key.ne(&usdt_receiver_pubkey) {
        msg!("invalid usdt receiver address");
        return Err(ProgramError::InvalidAccountData);
    }
    if presale_pda.owner.ne(program_id) {
        msg!("invalid presale_pda address");
        return Err(ProgramError::InvalidAccountData);
    }
    if config_pda.owner.ne(program_id) {
        msg!("invalid config_pda address");
        return Err(ProgramError::InvalidAccountData);
    }

    let clock = Clock::get()?;
    msg!("current_time: {:?}", clock.unix_timestamp);
    if clock.unix_timestamp > PRESALE_ENDTIME {
        return Err(ProgramError::Custom(501)); // presale is closed
    }
    if amount < MIN_USDT_AMOUNT {
        return Err(ProgramError::Custom(507)); // minimum presale price is too small
    }

    // usdt amount calc nest amount
    let nest_amount = (amount / RATIO) * RATIO_BOOST;

    // check nest balance
    let mut config_data = ConfigData::try_from_slice(&config_pda.data.borrow())?;
    let nest_balance = config_data.nest_total - config_data.presale_total;
    if nest_balance == 0 {
        return Err(ProgramError::Custom(508)); // presale is completed
    }
    if nest_amount > nest_balance {
        return Err(ProgramError::InsufficientFunds);
    }

    // calc nest cliff amount
    let nest_cliff = nest_amount * CLIFF_PERCENT / RATIO_BOOST;
    msg!(
        "usdt {} nest_total {} nest_cliff {}",
        amount,
        nest_amount,
        nest_cliff
    );

    // usdt transfer
    let transfer_ix = &spl_transfer(
        &spl_token::id(),
        &source_ata.key,
        &destination_ata.key,
        &owner_account.key,
        &[],
        amount,
    )?;
    invoke(
        &transfer_ix,
        &[
            token_program.clone(),
            source_ata.clone(),
            destination_ata.clone(),
            owner_account.clone(),
        ],
    )?;

    // payer presale_pda data save
    let mut presale_data = PresaleData::try_from_slice(&presale_pda.data.borrow())?;
    presale_data.nest_total += nest_amount;
    presale_data.nest_cliff += nest_cliff;
    presale_data.serialize(&mut *presale_pda.data.borrow_mut())?;
    msg!("presale_data {:?}", &presale_data);

    // config data update
    config_data.presale_total += nest_amount;
    config_data.serialize(&mut *config_pda.data.borrow_mut())?;
    msg!("config_data {:?}", &config_data);

    Ok(())
}

// user claim nest token
pub fn claim(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let token_program = next_account_info(accounts_iter)?; // token program id
    let owner_account = next_account_info(accounts_iter)?; // payer owner account
    let source_ata = next_account_info(accounts_iter)?; // bank_pda nest account
    let destination_ata = next_account_info(accounts_iter)?; // payer nest account
    let presale_pda = next_account_info(accounts_iter)?; // payer presale_pda account
    let bank_pda = next_account_info(accounts_iter)?; // bank_pda accoount

    if !owner_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    // check payer presale pda account
    let (presale_pda_pk, presale_bump) = Pubkey::find_program_address(
        &[
            PresaleData::SEED_PRESALE.as_bytes(),
            owner_account.key.as_ref(),
        ],
        program_id,
    );
    msg!("presale_pda account {:?} {}", presale_pda_pk, presale_bump);
    if presale_pda.key.ne(&presale_pda_pk) {
        msg!("owner presale_pda account mismatch");
        return Err(ProgramError::InvalidSeeds);
    }
    if presale_pda.owner.ne(program_id) {
        msg!("invalid presale_pda address");
        return Err(ProgramError::InvalidAccountData);
    }

    // check bank pda account
    let (bank_pda_pk, bank_bump) =
        Pubkey::find_program_address(&[PresaleData::SEED_BANK.as_bytes()], program_id);
    msg!("bank_pda account {:?} {}", bank_pda_pk, bank_bump);
    if bank_pda.key.ne(&bank_pda_pk) {
        msg!("invalid bank_pda account");
        return Err(ProgramError::InvalidSeeds);
    }

    let clock = Clock::get()?;
    msg!("current_time: {:?}", clock.unix_timestamp);
    if clock.unix_timestamp < UNLOCK_START_TIME {
        return Err(ProgramError::Custom(502)); // unlock not started
    }

    let mut presale_data = PresaleData::try_from_slice(&presale_pda.data.borrow())?;
    if presale_data.bump != presale_bump {
        msg!("invalid presale_pda account bump");
        return Err(ProgramError::InvalidSeeds);
    }
    let nest_total = presale_data.nest_total;
    if nest_total == 0 {
        msg!("presale nest amount is zero");
        return Err(ProgramError::Custom(503)); // presale nest amount is zero
    }

    let mut claim_amt = 0;
    let mut lock_period = 0;

    // claim cliff nest token
    let cliff_amt = presale_data.nest_cliff;
    if cliff_amt > 0 {
        claim_amt = cliff_amt;
        presale_data.nest_claim = cliff_amt;
        presale_data.nest_cliff = 0;
    }

    // claim unlock nest token
    let mut period_calc = ((clock.unix_timestamp - UNLOCK_START_TIME) as u64) / PER_MONTH_SECOND;
    if period_calc > 0 {
        if period_calc > PERIOD_TOTAL {
            period_calc = PERIOD_TOTAL;
        }
        msg!("period_calc: {}", period_calc);

        let pre_unlock_amt = nest_total * UNLOCK_PERCENT / RATIO_BOOST;
        let unlock_total = nest_total - presale_data.nest_claim;
        if unlock_total == 0 {
            return Err(ProgramError::Custom(504)); // claim is complete
        }

        let mut unlock_period = unlock_total / pre_unlock_amt;
        if unlock_period > PERIOD_TOTAL {
            unlock_period = PERIOD_TOTAL;
        }

        lock_period = PERIOD_TOTAL - unlock_period;
        let curr_period = period_calc - lock_period;
        if curr_period == 0 {
            return Err(ProgramError::Custom(505)); // claim period is zero
        }
        lock_period += curr_period;
        let curr_amt = pre_unlock_amt * curr_period;
        claim_amt += curr_amt;
        presale_data.nest_claim += curr_amt;

        if nest_total < presale_data.nest_claim {
            return Err(ProgramError::Custom(504)); // claim is complete
        }
    }

    if claim_amt == 0 {
        msg!("claim account is zero");
        return Err(ProgramError::Custom(506)); // claim account is zero
    }

    // nest transfer
    let transfer_ix = &spl_transfer(
        &spl_token::ID,
        &source_ata.key,
        &destination_ata.key,
        &bank_pda.key,
        &[],
        claim_amt,
    )?;
    invoke_signed(
        &transfer_ix,
        &[
            token_program.clone(),
            source_ata.clone(),
            destination_ata.clone(),
            bank_pda.clone(),
        ],
        &[&[PresaleData::SEED_BANK.as_bytes(), &[bank_bump]]],
    )?;

    // payer presale_pda data update
    presale_data.serialize(&mut *presale_pda.data.borrow_mut())?;
    msg!("presale_data {:?} period {}", &presale_data, lock_period);

    Ok(())
}

pub fn withdraw(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let token_program = next_account_info(accounts_iter)?; // token program id
    let owner_account = next_account_info(accounts_iter)?; // payer owner account
    let source_ata = next_account_info(accounts_iter)?; // bank_pda nest account
    let destination_ata = next_account_info(accounts_iter)?; // payer nest account
    let bank_pda = next_account_info(accounts_iter)?; // bank_pda accoount

    if amount == 0 {
        msg!("amount is zero");
        return Err(ProgramError::InvalidArgument);
    }

    // check admin
    let admin_account = Pubkey::from_str(ADMIN_ADDRESS).unwrap();
    if owner_account.key.ne(&admin_account) {
        return Err(ProgramError::InvalidAccountData);
    }

    // check bank pda account
    let (bank_pda_pk, bank_bump) =
        Pubkey::find_program_address(&[PresaleData::SEED_BANK.as_bytes()], program_id);
    msg!("bank_pda account {:?} {}", bank_pda_pk, bank_bump);
    if bank_pda.key.ne(&bank_pda_pk) {
        msg!("invalid bank_pda account");
        return Err(ProgramError::InvalidAccountData);
    }

    // nest transfer
    let transfer_ix = &spl_transfer(
        &spl_token::ID,
        &source_ata.key,
        &destination_ata.key,
        &bank_pda.key,
        &[],
        amount,
    )?;
    invoke_signed(
        &transfer_ix,
        &[
            token_program.clone(),
            source_ata.clone(),
            destination_ata.clone(),
            bank_pda.clone(),
        ],
        &[&[PresaleData::SEED_BANK.as_bytes(), &[bank_bump]]],
    )?;

    Ok(())
}
