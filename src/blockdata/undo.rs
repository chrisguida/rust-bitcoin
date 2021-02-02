// Rust Bitcoin Library
// Written in 2014 by
//     Andrew Poelstra <apoelstra@wpsoftware.net>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the CC0 Public Domain Dedication
// along with this software.
// If not, see <http://creativecommons.org/publicdomain/zero/1.0/>.
//

//! Undo
//!
//! Bitcoin's rev*.dat files store all of the spent transaction outputs for every
//! block
//!
//! This module provides the structures and functions needed to work with these files.
//!

use std::io;

// use hashes::{Hash, HashEngine};
// use hash_types::BlockHash;
// use util::uint::Uint256;
use consensus::encode::{self, CompressedScript, VarInt, VarInt2, Encodable, Decodable, ReadExt, Error};
// use network::constants::Network;
// use blockdata::transaction::Transaction;
// use blockdata::script;

/// A Bitcoin undo block, which is a collection of undo transactions, which
/// themeselves are 
#[derive(PartialEq, Eq, Clone, Debug)]
// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BlockUndo {
    /// List of undo transaction records, one for each transaction in the block
    /// excluding the coinbase transaction
    pub txdata_undo: Vec<TxUndo>
}

impl_consensus_encoding!(BlockUndo, txdata_undo);

impl BlockUndo {
    /// Get the size of the undo block
    pub fn get_size(&self) -> usize {
        // The size of the varint with the spent tx count + the spent txs themselves
        let tx_count_len = VarInt(self.txdata_undo.len() as u64).len();
        let txs_size: usize = self.txdata_undo.iter().map(TxUndo::get_size).sum();
        tx_count_len + txs_size
    }

}

/// A Bitcoin undo transaction, which is the reverse of a bitcoin transaction
#[derive(PartialEq, Eq, Clone, Debug)]
// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TxUndo {
    /// List of undo transaction input records, one for each utxo from the original transaction
    pub output_undo: Vec<TxOutUndo>
}

impl_consensus_encoding!(TxUndo, output_undo);

impl TxUndo {
    /// Get the size of the spent transaction
    pub fn get_size(&self) -> usize {
        todo!();
    }

}

/// A Bitcoin undo transaction input, which is the reverse of a transaction input
#[derive(PartialEq, Eq, Clone, Debug)]
// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TxOutUndo {
    /// Whether the spent output was a coinbase
    pub is_coin_base: bool,
    /// The height of the block in which the output was spent
    pub height: u64,
    /// The amount of the spent output
    pub amount: u64,
    /// The script of the spent output
    pub script_pubkey: CompressedScript,
}

// impl_consensus_encoding!(TxOutUndo, is_coin_base, height, output);

impl Encodable for TxOutUndo {
    fn consensus_encode<S: io::Write>(
        &self,
        s: S,
    ) -> Result<usize, encode::Error> {
        // let len = self.txid.consensus_encode(&mut s)?;
        Ok(self.is_coin_base.consensus_encode(s)?)
        // todo!();
    }
}

impl Decodable for TxOutUndo {
    fn consensus_decode<D: io::Read>(mut d: D) -> Result<Self, encode::Error> {
        // read height code, is (2 * (actual height) ) (+1 if coinbase)
        // let mut reader = Cursor::new(values);
        let height_code = VarInt2::consensus_decode(&mut d)?.0 as usize;
        let mut is_coin_base = false;
        if height_code % 2 == 1 {
            is_coin_base = true;
        }
        let height = (height_code / 2) as usize;
        // println!("found height: {}", height);

        // skip byte reserved only for backwards compatibility, should always be 0x00
        let _ = (&mut d).read_u8()?;

        // 
        let amount_compressed = VarInt2::consensus_decode(&mut d)?.0 as usize;
        let amount = decompress_txout_amt(amount_compressed)?;
        // println!("found amount: {}", amount);

        let script_len_code = VarInt2::consensus_decode(&mut d)?.0 as usize;
        let script_len = match script_len_code {
            0 | 1 => 20,
            2..=5 => 32,
            _ => script_len_code - 6
        };
        // println!("found script_len {}", script_len);
        // let mut script_pubkey_buf = Vec::with_capacity(script_len as usize);
        let mut script_pubkey_buf = vec![0u8; script_len + 1 as usize];
        script_pubkey_buf[0] = script_len_code as u8;
        // d.read_slice(&mut script_pubkey_buf)?;
        // let script_byte = (&mut d).read_u8()?;
        (&mut d).read_slice(&mut script_pubkey_buf[1..])?;
        // println!("found script_pubkey_buf {:?}", script_pubkey_buf);
        let script_pubkey = CompressedScript::consensus_decode(&mut std::io::Cursor::new(script_pubkey_buf)).unwrap();
        Ok(TxOutUndo {
            is_coin_base: is_coin_base,
            height: height as u64,
            amount: amount as u64,
            script_pubkey: script_pubkey,
        })
    }
}

impl TxOutUndo {
    /// Get the size of the spent txout
    pub fn get_size(&self) -> usize {
        todo!();
    }

}

fn decompress_txout_amt(mut value_compressed: usize) -> Result<usize, Error> {
    // (this function stolen from https://github.com/sr-gi/bitcoin_tools and ported to rust)
    // No need to do any work if it's zero.
    if value_compressed == 0 {
        return Ok(0);
    }

    // The decompressed amount is either of the following two equations:
    // x = 1 + 10*(9*n + d - 1) + e
    // x = 1 + 10*(n - 1)       + 9
    value_compressed -= 1;

    // The decompressed amount is now one of the following two equations:
    // x = 10*(9*n + d - 1) + e
    // x = 10*(n - 1)       + 9
    let exponent = value_compressed % 10;
    
    // integer division
    value_compressed /= 10;

    // The decompressed amount is now one of the following two equations:
    // x = 9*n + d - 1  | where e < 9
    // x = n - 1        | where e = 9
    // let mut n = 0 usize;
    let n: usize;
    if exponent < 9 {
        let last_digit = value_compressed % 9 + 1;
        // integer division
        value_compressed /= 9;
        n = value_compressed * 10 + last_digit;
    }
    else {
        n = value_compressed + 1;
    }

    // Apply the exponent.
    return Ok(n * 10usize.pow(exponent as u32))
}
