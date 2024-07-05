//! Error types that match their Haskell representation in the Cardano node

use pallas_codec::minicbor::{
    self,
    data::Type,
    decode::{Error, Token},
    Decode, Decoder,
};
use pallas_primitives::conway::ScriptHash;
use pallas_utxorpc::TxHash;

/// When decoding the error responses of the node, we use the following stack to track the location
/// of the decoding relative to an outer scope (most often a definite array).
type C = Vec<OuterScope>;

#[derive(Debug)]
pub struct TxApplyErrors {
    pub non_script_errors: Vec<ShelleyLedgerPredFailure>,
}

impl Decode<'_, C> for TxApplyErrors {
    fn decode(d: &mut Decoder, ctx: &mut C) -> Result<Self, Error> {
        let mut non_script_errors = vec![];
        expect_definite_array(vec![2], d, ctx)?;
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
        if let Err(_e) = expect_definite_array(vec![2], d, ctx) {
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
                    ctx.push(OuterScope::Indefinite);
                    clear_unknown_entity(d, ctx)?;
                }
                Token::Array(n) | Token::Map(n) => {
                    ctx.push(OuterScope::Definite(n));
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
    AlonzoInBabbageUtxowPredFailure(AlonzoUtxowPredFailure),
    UtxoFailure(BabbageUtxoPredFailure),
    MalformedScriptWitnesses,
    MalformedReferenceScripts,
}

impl Decode<'_, C> for BabbageUtxowPredFailure {
    fn decode(d: &mut Decoder, ctx: &mut C) -> Result<Self, Error> {
        expect_definite_array(vec![2], d, ctx)?;
        if let Ok(tag) = expect_u8(d, ctx) {
            match tag {
                1 => {
                    let utxo_failure = AlonzoUtxowPredFailure::decode(d, ctx)?;
                    Ok(BabbageUtxowPredFailure::AlonzoInBabbageUtxowPredFailure(
                        utxo_failure,
                    ))
                }
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
        expect_definite_array(vec![2], d, ctx)?;
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
    ValueNotConservedUTxO {
        consumed_value: pallas_primitives::conway::Value,
        produced_value: pallas_primitives::conway::Value,
    },
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
        let arr_len = expect_definite_array(vec![2, 3], d, ctx)?;
        if let Ok(tag) = expect_u8(d, ctx) {
            match tag {
                0 if arr_len == 2 => {
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
                5 if arr_len == 3 => {
                    // ValueNotConservedUtxo

                    let consumed_value = decode_conway_value(d, ctx)?;
                    let produced_value = decode_conway_value(d, ctx)?;

                    Ok(AlonzoUtxoPredFailure::ValueNotConservedUTxO {
                        consumed_value,
                        produced_value,
                    })
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

#[derive(Debug)]
pub enum AlonzoUtxowPredFailure {
    ShelleyInAlonzoUtxowPredfailure(ShelleyUtxowPredFailure),
    MissingRedeemers,
    MissingRequiredDatums,
    NotAllowedSupplementalDatums,
    PPViewHashesDontMatch,
    MissingRequiredSigners(Vec<pallas_crypto::hash::Hash<28>>),
    UnspendableUtxoNoDatumHash,
    ExtraRedeemers,
}

impl Decode<'_, C> for AlonzoUtxowPredFailure {
    fn decode(d: &mut Decoder, ctx: &mut C) -> Result<Self, Error> {
        expect_definite_array(vec![2], d, ctx)?;
        if let Ok(tag) = expect_u8(d, ctx) {
            match tag {
                0 => {
                    let shelley_utxow_failure = ShelleyUtxowPredFailure::decode(d, ctx)?;
                    Ok(AlonzoUtxowPredFailure::ShelleyInAlonzoUtxowPredfailure(
                        shelley_utxow_failure,
                    ))
                }
                5 => {
                    // MissingRequiredSigners
                    let signers: Result<Vec<_>, _> = d.array_iter()?.collect();
                    let signers = signers?;
                    Ok(AlonzoUtxowPredFailure::MissingRequiredSigners(signers))
                }
                //7 => {
                //    // ExtraRedeemers
                //}
                _ => Err(Error::message(format!(
                    "AlonzoUtxowPredFailure unhandled tag {}",
                    tag
                ))),
            }
        } else {
            add_collection_token_to_context(d, ctx)?;
            Err(Error::message(
                "AlonzoUtxoPredwFailure::decode: expected tag",
            ))
        }
    }
}

#[derive(Debug)]
pub enum ShelleyUtxowPredFailure {
    InvalidWitnessesUTXOW,
    /// Witnesses which failed in verifiedWits function
    MissingVKeyWitnessesUTXOW(Vec<pallas_crypto::hash::Hash<28>>),
    MissingScriptWitnessesUTXOW(Vec<ScriptHash>),
    ScriptWitnessNotValidatingUTXOW(Vec<ScriptHash>),
    UtxoFailure,
    MIRInsufficientGenesisSigsUTXOW,
    MissingTxBodyMetadataHash,
    MissingTxMetadata,
    ConflictingMetadataHash,
    InvalidMetadata,
    ExtraneousScriptWitnessesUTXOW(Vec<ScriptHash>),
}

impl Decode<'_, C> for ShelleyUtxowPredFailure {
    fn decode(d: &mut Decoder, ctx: &mut C) -> Result<Self, Error> {
        expect_definite_array(vec![2], d, ctx)?;
        if let Ok(tag) = expect_u8(d, ctx) {
            match tag {
                2 => {
                    let missing_script_witnesses: Result<Vec<_>, _> = d.array_iter()?.collect();
                    let missing_script_witnesses = missing_script_witnesses?;
                    Ok(ShelleyUtxowPredFailure::MissingScriptWitnessesUTXOW(
                        missing_script_witnesses,
                    ))
                }
                1 => {
                    // MissingVKeyWitnessesUTXOW
                    let missing_vkey_witnesses: Result<Vec<_>, _> = d.array_iter()?.collect();
                    let missing_vkey_witnesses = missing_vkey_witnesses?;
                    Ok(ShelleyUtxowPredFailure::MissingVKeyWitnessesUTXOW(
                        missing_vkey_witnesses,
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
pub struct TxInput {
    pub tx_hash: TxHash,
    pub index: u64,
}

impl Decode<'_, C> for TxInput {
    fn decode(d: &mut Decoder, ctx: &mut C) -> Result<Self, Error> {
        expect_definite_array(vec![2], d, ctx)?;
        if let Ok(bytes) = d.probe().bytes() {
            let _ = d.bytes()?;
            let tx_hash = TxHash::from(bytes);
            if let Ok(index) = d.probe().int() {
                if let Some(OuterScope::Definite(n)) = ctx.pop() {
                    if n > 1 {
                        ctx.push(OuterScope::Definite(n - 1));
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

/// Process the next CBOR token, adjusting the position if the outer scope is a definite array.
/// If this token represents a new collection, add new scope to the stack.
fn add_collection_token_to_context(d: &mut Decoder, ctx: &mut C) -> Result<(), Error> {
    let t = next_token(d)?;
    if let Some(OuterScope::Definite(n)) = ctx.pop() {
        if n > 1 {
            ctx.push(OuterScope::Definite(n - 1));
        }
    }
    match t {
        Token::BeginArray | Token::BeginBytes | Token::BeginMap => {
            ctx.push(OuterScope::Indefinite);
        }
        Token::Array(n) | Token::Map(n) => {
            ctx.push(OuterScope::Definite(n));
        }

        // Throw away the token (even break)
        _ => (),
    }
    Ok(())
}

fn expect_definite_array(
    possible_lengths: Vec<u64>,
    d: &mut Decoder,
    ctx: &mut C,
) -> Result<u64, Error> {
    if let Some(len) = d.probe().array()? {
        if let Some(OuterScope::Definite(inner_n)) = ctx.pop() {
            if inner_n > 1 {
                ctx.push(OuterScope::Definite(inner_n - 1));
            }
        }
        ctx.push(OuterScope::Definite(len));
        let _ = d.array()?;
        if possible_lengths.is_empty() || possible_lengths.contains(&len) {
            Ok(len)
        } else {
            Err(Error::message(format!(
                "Expected array({:?}), got array({})",
                possible_lengths, len
            )))
        }
    } else {
        let t = next_token(d)?;
        if let Some(OuterScope::Definite(n)) = ctx.pop() {
            if n > 1 {
                ctx.push(OuterScope::Definite(n - 1));
            }
        }
        match t {
            Token::BeginArray | Token::BeginBytes | Token::BeginMap => {
                ctx.push(OuterScope::Indefinite);
            }
            Token::Array(n) | Token::Map(n) => {
                ctx.push(OuterScope::Definite(n));
            }

            // Throw away the token (even break)
            _ => (),
        }
        Err(Error::message(format!(
            "Expected array({:?}), got indefinite array",
            possible_lengths,
        )))
    }
}

fn expect_u8(d: &mut Decoder, ctx: &mut C) -> Result<u8, Error> {
    if let Ok(value) = d.probe().u8() {
        if let Some(OuterScope::Definite(n)) = ctx.pop() {
            if n > 1 {
                ctx.push(OuterScope::Definite(n - 1));
            }
        }
        let _ = d.u8()?;
        Ok(value)
    } else {
        Err(Error::message("Expected u8"))
    }
}

fn expect_u64(d: &mut Decoder, ctx: &mut C) -> Result<u64, Error> {
    if let Ok(value) = d.probe().int() {
        if let Some(OuterScope::Definite(n)) = ctx.pop() {
            if n > 1 {
                ctx.push(OuterScope::Definite(n - 1));
            }
        }
        let _ = d.int()?;
        Ok(u64::try_from(value).map_err(|e| Error::message(e.to_string()))?)
    } else {
        Err(Error::message("Expected u8"))
    }
}

fn decode_conway_value(
    d: &mut Decoder,
    ctx: &mut C,
) -> Result<pallas_primitives::conway::Value, Error> {
    use pallas_primitives::conway::Value;
    if let Ok(dt) = d.datatype() {
        match dt {
            minicbor::data::Type::U8
            | minicbor::data::Type::U16
            | minicbor::data::Type::U32
            | minicbor::data::Type::U64 => {
                if let Some(OuterScope::Definite(n)) = ctx.pop() {
                    if n > 1 {
                        ctx.push(OuterScope::Definite(n - 1));
                    }
                }
                Ok(Value::Coin(d.decode_with(ctx)?))
            }
            minicbor::data::Type::Array => {
                expect_definite_array(vec![2], d, ctx)?;
                let coin = expect_u64(d, ctx)?;
                let multiasset = d.decode_with(ctx)?;
                Ok(pallas_primitives::conway::Value::Multiasset(
                    coin, multiasset,
                ))
            }
            _ => Err(minicbor::decode::Error::message(
                "unknown cbor data type for Alonzo Value enum",
            )),
        }
    } else {
        Err(Error::message("msg"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OuterScope {
    /// We are within a definite CBOR collection such as an array or map. The inner `u64` indicates
    /// the number of elements left to be processed within the collection.
    Definite(u64),
    /// We are within an indefinite collection.
    Indefinite,
}

fn clear_unknown_entity(decoder: &mut Decoder, stack: &mut Vec<OuterScope>) -> Result<(), Error> {
    while let Some(e) = stack.pop() {
        let t = next_token(decoder)?;

        match e {
            OuterScope::Definite(num_left) => {
                if num_left > 1 {
                    stack.push(OuterScope::Definite(num_left - 1));
                }
            }
            OuterScope::Indefinite => stack.push(OuterScope::Indefinite),
        }

        match t {
            Token::BeginArray | Token::BeginBytes | Token::BeginMap => {
                stack.push(OuterScope::Indefinite);
            }
            Token::Array(n) | Token::Map(n) => {
                stack.push(OuterScope::Definite(n));
            }

            Token::Break => {
                assert_eq!(e, OuterScope::Indefinite);
                assert_eq!(stack.pop(), Some(OuterScope::Indefinite));
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
            .array(1)? // dummy value
            .array(1)? // dummy value
            .array(1)? // dummy value
            .array(1)? // dummy value
            .array(1)? // dummy value
            .array(1)? // dummy value
            .u8(200)?
            .end()?;

        Ok(buffer)
    }

    #[test]
    fn test_decode_splash_bot_example() {
        let hex_str = "82028182059f820082018207830000000100028200820282018207820181820382018258200faddf00919ef15d38ac07684199e69be95a003a15f757bf77701072b050c1f500820082028201830500821a06760d80a1581cfd10da3e6a578708c877e14b6aaeda8dc3a36f666a346eec52a30b3aa14974657374746f6b656e1a0001fbd08200820282018200838258200faddf00919ef15d38ac07684199e69be95a003a15f757bf77701072b050c1f5008258205f85cf7db4713466bc8d9d32a84b5b6bfd2f34a76b5f8cf5a5cb04b4d6d6f0380082582096eb39b8d909373c8275c611fae63792f5e3d0a67c1eee5b3afb91fdcddc859100ff";
        let bytes = hex::decode(hex_str).unwrap();

        let mut decoder = Decoder::new(&bytes);
        let a = TxApplyErrors::decode(&mut decoder, &mut vec![]).unwrap();
        dbg!(a);
    }

    #[test]
    fn test_decode_splash_dao_example() {
        let hex_str = "82028182059f820082018200820281581cfdaaeb99e53be5f626fb210239ece94127401d7f395a097d0a5d18ef82008201820783000001000300820082018200820181581c28c58c07ecd2012c6c683b44ce9691ea9b0fdb9b868125a2ac29382382008201820581581c0bbd6545f014f95a65b9df462088c6600d9b2bb6cee3fe20b53241ea820082028201820782018182038201825820e54d54359cd0da7b5ee800c3c83b3f108894d4ef76bde10df66f87c429600e88018200820282018305821a002dc6c0a2581cadf2425c138138efce80fd0b2ed8f227caf052f9ec44b8a92e942dfaa14653504c4153481b00001d1a94a20000581cfdaaeb99e53be5f626fb210239ece94127401d7f395a097d0a5d18efa15820378d0caaaa3855f1b38693c1d6ef004fd118691c95c959d4efa950d6d6fcf7c101821a00765cada1581cadf2425c138138efce80fd0b2ed8f227caf052f9ec44b8a92e942dfaa14653504c4153481b00001d1a94a20000820082028201820081825820e54d54359cd0da7b5ee800c3c83b3f108894d4ef76bde10df66f87c429600e880182018201a1581de028c58c07ecd2012c6c683b44ce9691ea9b0fdb9b868125a2ac29382300ff";
        let bytes = hex::decode(hex_str).unwrap();

        let mut decoder = Decoder::new(&bytes);
        let a = TxApplyErrors::decode(&mut decoder, &mut vec![]).unwrap();
        dbg!(a);
    }
}
