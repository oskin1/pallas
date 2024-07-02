use pallas_codec::minicbor::{
    decode::{Error, Token, Tokenizer},
    Decoder,
};

use crate::miniprotocols::localstate::queries_v16::UTxO;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    Idle,
    Busy,
    Done,
}

#[derive(Debug)]
pub enum Message<Tx, Reject> {
    SubmitTx(Tx),
    AcceptTx,
    RejectTx(Reject),
    Done,
}

// The bytes of a transaction with an era number.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EraTx(pub u16, pub Vec<u8>);

// Raw reject reason.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RejectReason(pub Vec<u8>);

#[repr(u8)]
pub enum UtxoFailure {
    BadInputsUtxo(Vec<UTxO>),
    /// A generic error that cannot be clearly identified.
    Unknown(String),
}

pub fn process_reasons(
    RejectReason(raw_rejection_bytes): RejectReason,
) -> Result<Vec<UtxoFailure>, Error> {
    let mut reasons = vec![];
    let res: Result<Vec<Token>, Error> = Tokenizer::new(&raw_rejection_bytes).collect();
    match res {
        Ok(tokens) => {
            //
            Ok(reasons)
        }
        Err(e) => Err(e),
    }
}

fn process_single_reason<'a, T: Iterator<Item = Token<'a>>>(
    mut tokens: T,
) -> (Option<UtxoFailure>, T) {
    let t = tokens.next().unwrap();

    enum E {
        Definite(u64),
        Indefinite,
    }

    match t {
        Token::Break => (None, tokens),
        Token::Array(n) => {
            let mut stack = vec![E::Definite(n)];
            let mut subsequence = vec![Token::Array(n)];

            'outer: while let Some(remaining) = stack.pop() {
                match remaining {
                    E::Definite(elements_remaining) => {
                        let mut elements_remaining = elements_remaining;
                        while elements_remaining > 0 {
                            let t = tokens.next().unwrap();
                            subsequence.push(t);
                            match t {
                                Token::Array(n) => {
                                    stack.push(E::Definite(n));
                                    continue 'outer;
                                }
                                Token::Map(n) => {
                                    // 2*n because there are n key-value pairs.
                                    stack.push(E::Definite(2 * n));
                                    continue 'outer;
                                }
                                Token::BeginArray
                                | Token::BeginBytes
                                | Token::BeginMap
                                | Token::BeginString => {
                                    stack.push(E::Indefinite);
                                    continue 'outer;
                                }
                                _ => (),
                            }
                            elements_remaining -= 1;
                        }
                    }
                    E::Indefinite => {
                        while let Some(t) = tokens.next() {
                            subsequence.push(t);
                            match t {
                                Token::Break => continue 'outer,
                                Token::Array(n) => {
                                    stack.push(E::Definite(n));
                                    continue 'outer;
                                }
                                Token::Map(n) => {
                                    // 2*n because there are n key-value pairs.
                                    stack.push(E::Definite(2 * n));
                                    continue 'outer;
                                }
                                Token::BeginArray
                                | Token::BeginBytes
                                | Token::BeginMap
                                | Token::BeginString => {
                                    stack.push(E::Indefinite);
                                    continue 'outer;
                                }
                                _ => (),
                            }
                        }
                    }
                }
            }

            todo!()
        }
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {}
