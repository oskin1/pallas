//! Error types that match their Haskell representation in the Cardano node

use pallas_codec::minicbor::{
    data::Type,
    decode::{Error, Token},
    Decode, Decoder,
};
use pallas_utxorpc::TxHash;

type C = Vec<E>;

#[derive(Debug)]
pub struct TxApplyErrors {
    non_script_errors: Vec<ShelleyLedgerPredFailure>,
}

impl Decode<'_, C> for TxApplyErrors {
    fn decode(d: &mut Decoder, ctx: &mut C) -> Result<Self, Error> {
        let mut non_script_errors = vec![];
        expect_definite_array(2, d, ctx)?;
        let tag = expect_u8(d, ctx)?;
        assert_eq!(tag, 2);
        if let Some(n) = d.array()? {
            if n == 1 {
                let inner_n = d.array()?.unwrap();
                assert_eq!(inner_n, 2);

                // This tag is not totally understood.
                let _inner_tag = d.u8()?;

                // Here we expect an indefinite array
                if d.array()?.is_none() {
                    while d.datatype().is_ok() {
                        if let Ok(err) = ShelleyLedgerPredFailure::decode(d, ctx) {
                            ctx.clear();
                            dbg!(&err);
                            non_script_errors.push(err);
                        }
                    }
                }

                Ok(Self { non_script_errors })
            } else {
                Err(Error::message("TxApplyErrors::decode: expected array(1)"))
            }
        } else {
            Err(Error::message("TxApplyErrors::decode: expected array(1)"))
        }
    }
}

#[derive(Debug)]
/// Top level type for ledger errors
pub enum ShelleyLedgerPredFailure {
    UtxowFailure(BabbageUtxowPredFailure),
    DelegsFailure,
}

impl Decode<'_, C> for ShelleyLedgerPredFailure {
    fn decode(d: &mut Decoder, ctx: &mut C) -> Result<Self, Error> {
        if let Err(_e) = expect_definite_array(2, d, ctx) {
            clear_unknown_entity(d, ctx)?;
        }
        if let Ok(tag) = expect_u8(d, ctx) {
            match tag {
                0 => match BabbageUtxowPredFailure::decode(d, ctx) {
                    Ok(utxow_failure) => Ok(ShelleyLedgerPredFailure::UtxowFailure(utxow_failure)),
                    Err(e) => {
                        clear_unknown_entity(d, ctx)?;
                        Err(e)
                    }
                },
                _ => {
                    clear_unknown_entity(d, ctx)?;
                    Err(Error::message("not ShelleyLedgerPredFailure"))
                }
            }
        } else {
            let t = next_token(d)?;
            match t {
                Token::BeginArray | Token::BeginBytes | Token::BeginMap => {
                    ctx.push(E::Indefinite);
                    clear_unknown_entity(d, ctx)?;
                }
                Token::Array(n) | Token::Map(n) => {
                    ctx.push(E::Definite(n));
                    clear_unknown_entity(d, ctx)?;
                }

                // Throw away the token (even break)
                _ => (),
            }
            Err(Error::message(
                "ShelleyLedgerPredFailure::decode: expected tag",
            ))
        }
    }
}

#[derive(Debug)]
pub enum BabbageUtxowPredFailure {
    AlonzoInBabbageUtxowPredFailure,
    UtxoFailure(BabbageUtxoPredFailure),
    MalformedScriptWitnesses,
    MalformedReferenceScripts,
}

impl Decode<'_, C> for BabbageUtxowPredFailure {
    fn decode(d: &mut Decoder, ctx: &mut C) -> Result<Self, Error> {
        expect_definite_array(2, d, ctx)?;
        if let Ok(tag) = expect_u8(d, ctx) {
            match tag {
                2 => {
                    let utxo_failure = BabbageUtxoPredFailure::decode(d, ctx)?;
                    Ok(BabbageUtxowPredFailure::UtxoFailure(utxo_failure))
                }
                _ => Err(Error::message("not BabbageUtxowPredFailure")),
            }
        } else {
            add_collection_token_to_context(d, ctx)?;
            Err(Error::message(
                "BabbageUtxowPredFailure::decode: expected tag",
            ))
        }
    }
}

#[derive(Debug)]
pub enum BabbageUtxoPredFailure {
    AlonzoInBabbageUtxoPredFailure(AlonzoUtxoPredFailure),
    IncorrectTotalCollateralField,
    BabbageOutputTooSmallUTxO,
    BabbageNonDisjointRefInputs,
}

impl Decode<'_, C> for BabbageUtxoPredFailure {
    fn decode(d: &mut Decoder, ctx: &mut C) -> Result<Self, Error> {
        expect_definite_array(2, d, ctx)?;
        if let Ok(tag) = expect_u8(d, ctx) {
            match tag {
                1 => {
                    let alonzo_failure = AlonzoUtxoPredFailure::decode(d, ctx)?;
                    Ok(BabbageUtxoPredFailure::AlonzoInBabbageUtxoPredFailure(
                        alonzo_failure,
                    ))
                }
                _ => Err(Error::message("not BabbageUtxoPredFailure")),
            }
        } else {
            add_collection_token_to_context(d, ctx)?;
            Err(Error::message(
                "BabbageUtxoPredFailure::decode: expected tag",
            ))
        }
    }
}

#[derive(Debug)]
pub enum AlonzoUtxoPredFailure {
    BadInputsUtxo(Vec<TxInput>),
    OutsideValidityIntervalUTxO,
    MaxTxSizeUTxO,
    InputSetEmptyUTxO,
    FeeTooSmallUTxO,
    ValueNotConservedUTxO,
    WrongNetwork,
    WrongNetworkWithdrawal,
    OutputTooSmallUTxO,
    UtxosFailure,
    OutputBootAddrAttrsTooBig,
    TriesToForgeADA,
    OutputTooBigUTxO,
    InsufficientCollateral,
    ScriptsNotPaidUTxO,
    ExUnitsTooBigUTxO,
    CollateralContainsNonADA,
    WrongNetworkInTxBody,
    OutsideForecast,
    TooManyCollateralInputs,
    NoCollateralInputs,
}

impl Decode<'_, C> for AlonzoUtxoPredFailure {
    fn decode(d: &mut Decoder, ctx: &mut C) -> Result<Self, Error> {
        expect_definite_array(2, d, ctx)?;
        if let Ok(tag) = expect_u8(d, ctx) {
            match tag {
                0 => {
                    // BadInputsUtxo
                    if let Some(num_bad_inputs) = d.array()? {
                        let mut bad_inputs = vec![];
                        for _ in 0..num_bad_inputs {
                            let tx_input = TxInput::decode(d, ctx)?;
                            bad_inputs.push(tx_input);
                        }
                        Ok(AlonzoUtxoPredFailure::BadInputsUtxo(bad_inputs))
                    } else {
                        Err(Error::message("expected array of tx inputs"))
                    }
                }
                _ => Err(Error::message("not AlonzoUtxoPredFailure")),
            }
        } else {
            add_collection_token_to_context(d, ctx)?;
            Err(Error::message(
                "AlonzoUtxoPredFailure::decode: expected tag",
            ))
        }
    }
}

pub enum AlonzoUtxowPredFailure {
    ShelleyInAlonzoUtxowPredfailure(ShelleyUtxowPredFailure),
    MissingRequiredDatums,
    NotAllowedSupplementalDatums,
    PPViewHashesDontMatch,
    MissingRequiredSigners,
    UnspendableUtxoNoDatumHash,
    ExtraRedeemers,
}

pub enum ShelleyUtxowPredFailure {}

#[derive(Debug)]
struct TxInput {
    tx_hash: TxHash,
    index: u64,
}

impl Decode<'_, C> for TxInput {
    fn decode(d: &mut Decoder, ctx: &mut C) -> Result<Self, Error> {
        expect_definite_array(2, d, ctx)?;
        if let Ok(bytes) = d.probe().bytes() {
            let _ = d.bytes()?;
            let tx_hash = TxHash::from(bytes);
            if let Ok(index) = d.probe().int() {
                if let Some(E::Definite(n)) = ctx.pop() {
                    if n > 1 {
                        ctx.push(E::Definite(n - 1));
                    }
                }
                let _ = d.int()?;
                let index =
                    u64::try_from(index).map_err(|_| Error::message("Can't convert Int to u64"))?;
                Ok(TxInput { tx_hash, index })
            } else {
                add_collection_token_to_context(d, ctx)?;
                Err(Error::message("TxInput::decode: expected index (int)"))
            }
        } else {
            Err(Error::message("TxInput::decode: expected bytes"))
        }
    }
}

fn add_collection_token_to_context(d: &mut Decoder, ctx: &mut C) -> Result<(), Error> {
    let t = next_token(d)?;
    if let Some(E::Definite(n)) = ctx.pop() {
        if n > 1 {
            ctx.push(E::Definite(n - 1));
        }
    }
    match t {
        Token::BeginArray | Token::BeginBytes | Token::BeginMap => {
            ctx.push(E::Indefinite);
        }
        Token::Array(n) | Token::Map(n) => {
            ctx.push(E::Definite(n));
        }

        // Throw away the token (even break)
        _ => (),
    }
    Ok(())
}

fn expect_definite_array(n: u64, d: &mut Decoder, ctx: &mut C) -> Result<(), Error> {
    if let Some(len) = d.probe().array()? {
        if let Some(E::Definite(n)) = ctx.pop() {
            if n > 1 {
                ctx.push(E::Definite(n - 1));
            }
        }
        ctx.push(E::Definite(len));
        let _ = d.array()?;
        if n == len {
            Ok(())
        } else {
            Err(Error::message(format!(
                "Expected array({}), got array({})",
                n, len
            )))
        }
    } else {
        let t = next_token(d)?;
        if let Some(E::Definite(n)) = ctx.pop() {
            if n > 1 {
                ctx.push(E::Definite(n - 1));
            }
        }
        match t {
            Token::BeginArray | Token::BeginBytes | Token::BeginMap => {
                ctx.push(E::Indefinite);
            }
            Token::Array(n) | Token::Map(n) => {
                ctx.push(E::Definite(n));
            }

            // Throw away the token (even break)
            _ => (),
        }
        Err(Error::message(format!(
            "Expected array({}), got indefinite array",
            n
        )))
    }
}

fn expect_u8(d: &mut Decoder, ctx: &mut C) -> Result<u8, Error> {
    if let Ok(value) = d.probe().u8() {
        if let Some(E::Definite(n)) = ctx.pop() {
            if n > 1 {
                ctx.push(E::Definite(n - 1));
            }
        }
        let _ = d.u8()?;
        Ok(value)
    } else {
        Err(Error::message("Expected u8"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum E {
    Definite(u64),
    Indefinite,
}

fn clear_unknown_entity(decoder: &mut Decoder, stack: &mut Vec<E>) -> Result<(), Error> {
    while let Some(e) = stack.pop() {
        let t = next_token(decoder)?;

        match e {
            E::Definite(num_left) => {
                if num_left > 1 {
                    stack.push(E::Definite(num_left - 1));
                }
            }
            E::Indefinite => stack.push(E::Indefinite),
        }

        match t {
            Token::BeginArray | Token::BeginBytes | Token::BeginMap => {
                stack.push(E::Indefinite);
            }
            Token::Array(n) | Token::Map(n) => {
                stack.push(E::Definite(n));
            }

            Token::Break => {
                dbg!(&stack);
                assert_eq!(e, E::Indefinite);
                assert_eq!(stack.pop(), Some(E::Indefinite));
            }

            // Throw away the token
            _ => (),
        }
    }
    Ok(())
}

fn next_token<'a>(decoder: &'a mut Decoder) -> Result<Token<'a>, Error> {
    match decoder.datatype()? {
        Type::Bool => decoder.bool().map(Token::Bool),
        Type::U8 => decoder.u8().map(Token::U8),
        Type::U16 => decoder.u16().map(Token::U16),
        Type::U32 => decoder.u32().map(Token::U32),
        Type::U64 => decoder.u64().map(Token::U64),
        Type::I8 => decoder.i8().map(Token::I8),
        Type::I16 => decoder.i16().map(Token::I16),
        Type::I32 => decoder.i32().map(Token::I32),
        Type::I64 => decoder.i64().map(Token::I64),
        Type::Int => decoder.int().map(Token::Int),
        Type::F16 => decoder.f16().map(Token::F16),
        Type::F32 => decoder.f32().map(Token::F32),
        Type::F64 => decoder.f64().map(Token::F64),
        Type::Bytes => decoder.bytes().map(Token::Bytes),
        Type::String => decoder.str().map(Token::String),
        Type::Tag => decoder.tag().map(Token::Tag),
        Type::Simple => decoder.simple().map(Token::Simple),
        Type::Array => {
            let p = decoder.position();
            if let Some(n) = decoder.array()? {
                Ok(Token::Array(n))
            } else {
                Err(Error::type_mismatch(Type::Array)
                    .at(p)
                    .with_message("missing array length"))
            }
        }
        Type::Map => {
            let p = decoder.position();
            if let Some(n) = decoder.map()? {
                Ok(Token::Map(n))
            } else {
                Err(Error::type_mismatch(Type::Array)
                    .at(p)
                    .with_message("missing map length"))
            }
        }
        Type::BytesIndef => {
            decoder.set_position(decoder.position() + 1);
            Ok(Token::BeginBytes)
        }
        Type::StringIndef => {
            decoder.set_position(decoder.position() + 1);
            Ok(Token::BeginString)
        }
        Type::ArrayIndef => {
            decoder.set_position(decoder.position() + 1);
            Ok(Token::BeginArray)
        }
        Type::MapIndef => {
            decoder.set_position(decoder.position() + 1);
            Ok(Token::BeginMap)
        }
        Type::Null => {
            decoder.set_position(decoder.position() + 1);
            Ok(Token::Null)
        }
        Type::Undefined => {
            decoder.set_position(decoder.position() + 1);
            Ok(Token::Undefined)
        }
        Type::Break => {
            decoder.set_position(decoder.position() + 1);
            Ok(Token::Break)
        }
        t @ Type::Unknown(_) => Err(Error::type_mismatch(t)
            .at(decoder.position())
            .with_message("unknown cbor type")),
    }
}

#[cfg(test)]
mod tests {
    use pallas_codec::minicbor::{
        encode::{write::EndOfSlice, Error},
        Decode, Decoder, Encoder,
    };

    use super::TxApplyErrors;

    #[test]
    fn test_decode_malformed_error() {
        let buffer = encode_trace().unwrap();
        let mut decoder = Decoder::new(&buffer);
        let a = TxApplyErrors::decode(&mut decoder, &mut vec![]).unwrap();
        dbg!(&a);
        assert!(a.non_script_errors.is_empty());
    }

    fn encode_trace() -> Result<[u8; 1280], Error<EndOfSlice>> {
        let mut buffer = [0u8; 1280];
        let mut encoder = Encoder::new(&mut buffer[..]);

        let _e = encoder
            .array(2)?
            .u8(2)?
            .array(1)?
            .array(2)?
            .u8(5)?
            .begin_array()?
            // Encode ledger errors
            .array(2)?
            .u8(0)? // Tag for BabbageUtxowPredFailure
            .array(2)?
            .u8(2)? // Tag for BabbageUtxoPredFailure
            .array(2)?
            .u8(1)? // Tag for AlonzoUtxoPredFailure
            .array(2)?
            .u8(100)? // Unsupported Tag
            .u8(200)? // dummy value
            .end()?;

        Ok(buffer)
    }

    #[test]
    fn test_decode_whole_response() {
        let hex_str = "82028182059f820082018207830000000100028200820282018207820181820382018258200faddf00919ef15d38ac07684199e69be95a003a15f757bf77701072b050c1f500820082028201830500821a06760d80a1581cfd10da3e6a578708c877e14b6aaeda8dc3a36f666a346eec52a30b3aa14974657374746f6b656e1a0001fbd08200820282018200838258200faddf00919ef15d38ac07684199e69be95a003a15f757bf77701072b050c1f5008258205f85cf7db4713466bc8d9d32a84b5b6bfd2f34a76b5f8cf5a5cb04b4d6d6f0380082582096eb39b8d909373c8275c611fae63792f5e3d0a67c1eee5b3afb91fdcddc859100ff";
        let bytes = hex::decode(hex_str).unwrap();

        let mut decoder = Decoder::new(&bytes);
        let a = TxApplyErrors::decode(&mut decoder, &mut vec![]).unwrap();
        dbg!(a);
    }
}
