// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {ITradeEdgeContract} from "./IContract.sol";

contract TradeEdgeContract is ITradeEdgeContract {
    address immutable OWNER;
    uint256 next;

    constructor() {
        OWNER = msg.sender;
        next = 0;
    }

    function emitTrade() public {
        require(msg.sender == OWNER);

        uint256 id = next;
        next = id + 1;

        bytes32 hash = keccak256(
            abi.encodePacked(
                bytes32(id) & bytes32(block.number)
                    & bytes32(uint256(75355315776851894333550748443319825964622367698396525930962805827881766561886))
            )
        );

        emit Trade(id, hash);
    }
}
