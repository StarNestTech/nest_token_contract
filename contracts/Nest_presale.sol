// SPDX-License-Identifier: MIT
// Compatible with OpenZeppelin Contracts ^5.0.0
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

contract NestPresale is Ownable {
    uint256 public constant RATIO_BOOST = 10000;
    uint256 public constant PER_MONTH_SECOND = 60 * 60 * 24 * 30;
    uint8 public constant TOTAL_PERIOD = 12;

    // 1USDT*Decimals/(20*NEST*Decimals)
    // 0.05USDT/NEST
    // eth 5 - 1e6/(20*1e8)*10000
    // bsc 5000000000000 - 1e18/(20*1e8)*10000
    uint256 public ratio;

    IERC20 public usdtCont; // usdt erc20 contract address
    IERC20 public nestCont; // nest erc20 contract address
    address public fundAddr; // fund address
    uint256 public endTime; // presale end time second
    uint256 public totalUsdt; // total presale USDT amount
    uint256 public presaledNest; // presaled total Nest amount
    uint256 public presaledUsdt; // presaled total USDT amount
    uint256 public minUsdtAmt; // minimum presale price
    uint256 public cliffPercent; // cliff percent
    uint256 public unlockPercent; // unlock percent
    uint256 public unlockStartTime; // unlock start timestamp

    mapping(address => uint256) public presaleTotal; // account presaled Nest amount
    mapping(address => uint256) public claimTotal; // account claimed Nest amount
    mapping(address => bool) public claimPresaled; // account is claimed presale Nest amount

    event UsdtCont(address _oldAddr, address _newAddr);
    event NestCont(address _oldAddr, address _newAddr);
    event FundAddr(address _oldAddr, address _newAddr);
    event Ratio(uint256 _oldRatio, uint256 _newRatio);
    event EndTime(uint256 _oldTime, uint256 _newTime);
    event TotalUsdt(uint256 _oldTotal, uint256 _newTotal);
    event MinUsdtAmount(uint256 _oldAmt, uint256 _newAmt);
    event CliffPercent(uint256 _oldPercent, uint256 _newPercent);
    event UnlockPercent(uint256 _oldPercent, uint256 _newPercent);
    event Presale(address _acct, uint256 _usdtAmt, uint256 _nestTotal);
    event Claim(address _acct, uint8 _period, uint256 _nestUnlock);
    event UnlockStartTime(uint256 _time);

    constructor(
        address initialOwner,
        address _usdt,
        address _nest,
        uint256 _ratio,
        address _fund,
        uint256 _totalUsdt,
        uint256 _endTime,
        uint256 _minUsdtAmt
    ) Ownable(initialOwner) {
        require(_usdt != address(0), "invalid USDT address");
        require(_nest != address(0), "invalid Nest address");
        require(_ratio > 0, "invalid ratio");
        require(_fund != address(0), "invalid fund address");
        require(_totalUsdt > 0, "invalid total USDT amount");
        require(_endTime > block.timestamp, "invalid endTime");
        require(_minUsdtAmt > 0, "invalid minimum USDT amount");

        usdtCont = IERC20(_usdt);
        nestCont = IERC20(_nest);
        ratio = _ratio;
        fundAddr = _fund;
        totalUsdt = _totalUsdt;
        endTime = _endTime;
        minUsdtAmt = _minUsdtAmt;
        // 10+7.5*12=100
        cliffPercent = 1000;
        unlockPercent = 750;
    }

    function setUsdt(address _usdt) external onlyOwner {
        require(_usdt != address(0), "invalid usdt address");
        emit UsdtCont(address(usdtCont), _usdt);
        usdtCont = IERC20(_usdt);
    }

    function setNest(address _nest) external onlyOwner {
        require(_nest != address(0), "invalid Nest address");
        emit NestCont(address(nestCont), _nest);
        nestCont = IERC20(_nest);
    }

    // RATIO_BOOST
    function setRatio(uint256 _ratio) external onlyOwner {
        require(_ratio > 0, "invalid ratio");
        emit Ratio(ratio, _ratio);
        ratio = _ratio;
    }

    function setFund(address _fund) external onlyOwner {
        require(_fund != address(0), "invalid fund address");
        emit FundAddr(fundAddr, _fund);
        fundAddr = _fund;
    }

    function setEndTime(uint256 _endTime) external onlyOwner {
        require(_endTime > block.timestamp, "invalid endTime");
        emit EndTime(endTime, _endTime);
        endTime = _endTime;
    }

    function setTotalUsdt(uint256 _totalAmt) external onlyOwner {
        require(_totalAmt > 0, "invalid total USDT Amount");
        emit TotalUsdt(totalUsdt, _totalAmt);
        totalUsdt = _totalAmt;
    }

    function setCliffPercent(uint256 _percent) external onlyOwner {
        require(_percent > 0, "invalid cliff percent");
        emit CliffPercent(cliffPercent, _percent);
        cliffPercent = _percent;
    }

    function setUnlockPercent(uint256 _percent) external onlyOwner {
        require(_percent > 0, "invalid unlock percent");
        emit UnlockPercent(unlockPercent, _percent);
        unlockPercent = _percent;
    }

    function setMinUsdtAmt(uint256 _usdtAmt) external onlyOwner {
        require(_usdtAmt > 0, "invalid minimum USDT amount");
        emit MinUsdtAmount(minUsdtAmt, _usdtAmt);
        minUsdtAmt = _usdtAmt;
    }

    function setUnlockStartTime(uint256 _time) external onlyOwner {
        require(_time > block.timestamp, "invalid unlock start timestamp");
        emit UnlockStartTime(_time);
        unlockStartTime = _time;
    }

    // account presale Nest token
    // _amount: usdt amount
    function presale(uint256 _usdtAmt) external {
        require(_usdtAmt >= minUsdtAmt, "minimum presale price is too small");
        require(block.timestamp <= endTime, "presale is end");
        presaledUsdt += _usdtAmt;
        require(totalUsdt - presaledUsdt >= 0, "presale insufficient amount");

        bool isOk = usdtCont.transferFrom(msg.sender, fundAddr, _usdtAmt);
        require(isOk, "usdt transfer fail");

        uint256 _nestTotal = (_usdtAmt / ratio) * RATIO_BOOST;
        require(_nestTotal > 0, "Nest amount is zero");

        uint256 _nestCliff = (_nestTotal * cliffPercent) / RATIO_BOOST;
        require(_nestCliff > 0, "Nest cliff amount is zero");

        presaledNest += _nestTotal;
        presaleTotal[msg.sender] += _nestTotal;

        emit Presale(msg.sender, _usdtAmt, _nestTotal);
    }

    // account claim Nest token
    function claim() external {
        require(block.timestamp >= unlockStartTime && unlockStartTime > 0, "unlock not started");
        uint256 _nestTotal = presaleTotal[msg.sender];
        require(_nestTotal > 0, "Presale Nest amount is zero");

        uint256 _cliamAmt;
        uint8 _lockPeriod;
        // first claim presale 10% Nest token
        if (!claimPresaled[msg.sender]) {
            _cliamAmt = (_nestTotal * cliffPercent) / RATIO_BOOST;
            claimTotal[msg.sender] = _cliamAmt;
            claimPresaled[msg.sender] = true;
        }

        // other period unlock 7.5% Nest token
        uint8 _period = uint8((block.timestamp - unlockStartTime) / PER_MONTH_SECOND);
        if (_period > 0) {
            if (_period > TOTAL_PERIOD) {
                _period = TOTAL_PERIOD;
            }

            uint256 _perUnlock = (_nestTotal * unlockPercent) / RATIO_BOOST;
            uint256 _unlockAmt = _nestTotal - claimTotal[msg.sender];
            require(_unlockAmt > 0, "claim is complete");
            uint8 _unlockPeriod = uint8(_unlockAmt / _perUnlock);
            if (_unlockPeriod > TOTAL_PERIOD) {
                _unlockPeriod = TOTAL_PERIOD;
            }

            _lockPeriod = (TOTAL_PERIOD - _unlockPeriod);
            uint256 _currPeriod = _period - _lockPeriod;
            require(_currPeriod > 0, "claim period is zero");
            _lockPeriod += uint8(_currPeriod);
            uint256 _currAmt = _perUnlock * _currPeriod;
            claimTotal[msg.sender] += _currAmt;
            _cliamAmt += _currAmt;
        }
        require(_cliamAmt > 0, "cliam amount is zero");

        bool isSucc = nestCont.transfer(msg.sender, _cliamAmt);
        require(isSucc, "Nest transfer fail");

        emit Claim(msg.sender, _lockPeriod, _cliamAmt);
    }

    function withdraw(address _contAddr) external onlyOwner {
        require(_contAddr != address(0), "invalid contract address");
        if (_contAddr == address(1)) {
            uint256 _balance = address(this).balance;
            require(_balance > 0, "insufficient amount");
            payable(msg.sender).transfer(_balance);
        } else {
            IERC20 _erc20 = IERC20(_contAddr);
            uint256 _balance = _erc20.balanceOf(address(this));
            require(_balance > 0, "insufficient amount");
            _erc20.transfer(msg.sender, _balance);
        }
    }

    receive() external payable {}

    fallback() external payable {}
}
