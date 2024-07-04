use pallas_codec::minicbor::{Decode, Decoder};
use pallas_crypto::hash::Hash;
use pallas_primitives::conway::Value;

use crate::miniprotocols::localtxsubmission::cardano_node_errors::TxApplyErrors;

use super::cardano_node_errors::{
    AlonzoUtxoPredFailure, AlonzoUtxowPredFailure, BabbageUtxoPredFailure, BabbageUtxowPredFailure,
    ShelleyLedgerPredFailure, ShelleyUtxowPredFailure, TxInput,
};

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
pub struct RejectReasonBytes(pub Vec<u8>);

pub struct TxRejectionReasons(Vec<RejectReason>);

impl TryFrom<RejectReasonBytes> for TxRejectionReasons {
    type Error = pallas_codec::minicbor::decode::Error;

    fn try_from(
        RejectReasonBytes(raw_rejection_bytes): RejectReasonBytes,
    ) -> Result<Self, Self::Error> {
        let mut decoder = Decoder::new(&raw_rejection_bytes);
        let node_errors = TxApplyErrors::decode(&mut decoder, &mut vec![])?;

        let reasons: Vec<_> = node_errors
            .non_script_errors
            .into_iter()
            .map(RejectReason::from)
            .collect();
        Ok(TxRejectionReasons(reasons))
    }
}

pub enum RejectReason {
    /// Missing/bad TX inputs
    BadInputsUtxo(Vec<TxInput>),
    ValueNotConserved {
        consumed: Value,
        produced: Value,
    },
    MissingVKeyWitnesses(Vec<Hash<28>>),
    MissingScriptWitnesses(Vec<Hash<28>>),
    /// A generic error that cannot be clearly identified.
    Unknown(String),
}

impl From<ShelleyLedgerPredFailure> for RejectReason {
    fn from(value: ShelleyLedgerPredFailure) -> Self {
        match value {
            ShelleyLedgerPredFailure::UtxowFailure(babbage_failure) => match babbage_failure {
                BabbageUtxowPredFailure::AlonzoInBabbageUtxowPredFailure(alonzo_fail) => {
                    match alonzo_fail {
                        AlonzoUtxowPredFailure::ShelleyInAlonzoUtxowPredfailure(shelley_fail) => {
                            match shelley_fail {
                                ShelleyUtxowPredFailure::MissingVKeyWitnessesUTXOW(m) => {
                                    RejectReason::MissingVKeyWitnesses(m)
                                }
                                ShelleyUtxowPredFailure::MissingScriptWitnessesUTXOW(m) => {
                                    RejectReason::MissingScriptWitnesses(m)
                                }
                                ShelleyUtxowPredFailure::InvalidWitnessesUTXOW => unimplemented!(),
                                ShelleyUtxowPredFailure::ScriptWitnessNotValidatingUTXOW(_) => {
                                    unimplemented!()
                                }
                                ShelleyUtxowPredFailure::UtxoFailure => unimplemented!(),
                                ShelleyUtxowPredFailure::MIRInsufficientGenesisSigsUTXOW => {
                                    unimplemented!()
                                }
                                ShelleyUtxowPredFailure::MissingTxBodyMetadataHash => {
                                    unimplemented!()
                                }
                                ShelleyUtxowPredFailure::MissingTxMetadata => unimplemented!(),
                                ShelleyUtxowPredFailure::ConflictingMetadataHash => {
                                    unimplemented!()
                                }
                                ShelleyUtxowPredFailure::InvalidMetadata => unimplemented!(),
                                ShelleyUtxowPredFailure::ExtraneousScriptWitnessesUTXOW(_) => {
                                    unimplemented!()
                                }
                            }
                        }
                        AlonzoUtxowPredFailure::MissingRedeemers => unimplemented!(),
                        AlonzoUtxowPredFailure::MissingRequiredDatums => unimplemented!(),
                        AlonzoUtxowPredFailure::NotAllowedSupplementalDatums => unimplemented!(),
                        AlonzoUtxowPredFailure::PPViewHashesDontMatch => unimplemented!(),
                        AlonzoUtxowPredFailure::MissingRequiredSigners(_) => unimplemented!(),
                        AlonzoUtxowPredFailure::UnspendableUtxoNoDatumHash => unimplemented!(),
                        AlonzoUtxowPredFailure::ExtraRedeemers => unimplemented!(),
                    }
                }
                BabbageUtxowPredFailure::UtxoFailure(utxo_fail) => match utxo_fail {
                    BabbageUtxoPredFailure::AlonzoInBabbageUtxoPredFailure(alonzo_fail) => {
                        match alonzo_fail {
                            AlonzoUtxoPredFailure::BadInputsUtxo(inputs) => {
                                RejectReason::BadInputsUtxo(inputs)
                            }
                            AlonzoUtxoPredFailure::ValueNotConservedUTxO {
                                consumed_value,
                                produced_value,
                            } => RejectReason::ValueNotConserved {
                                consumed: consumed_value,
                                produced: produced_value,
                            },
                            AlonzoUtxoPredFailure::OutsideValidityIntervalUTxO => unimplemented!(),
                            AlonzoUtxoPredFailure::MaxTxSizeUTxO => unimplemented!(),
                            AlonzoUtxoPredFailure::InputSetEmptyUTxO => unimplemented!(),
                            AlonzoUtxoPredFailure::FeeTooSmallUTxO => unimplemented!(),
                            AlonzoUtxoPredFailure::WrongNetwork => unimplemented!(),
                            AlonzoUtxoPredFailure::WrongNetworkWithdrawal => unimplemented!(),
                            AlonzoUtxoPredFailure::OutputTooSmallUTxO => unimplemented!(),
                            AlonzoUtxoPredFailure::UtxosFailure => unimplemented!(),
                            AlonzoUtxoPredFailure::OutputBootAddrAttrsTooBig => unimplemented!(),
                            AlonzoUtxoPredFailure::TriesToForgeADA => unimplemented!(),
                            AlonzoUtxoPredFailure::OutputTooBigUTxO => unimplemented!(),
                            AlonzoUtxoPredFailure::InsufficientCollateral => unimplemented!(),
                            AlonzoUtxoPredFailure::ScriptsNotPaidUTxO => unimplemented!(),
                            AlonzoUtxoPredFailure::ExUnitsTooBigUTxO => unimplemented!(),
                            AlonzoUtxoPredFailure::CollateralContainsNonADA => unimplemented!(),
                            AlonzoUtxoPredFailure::WrongNetworkInTxBody => unimplemented!(),
                            AlonzoUtxoPredFailure::OutsideForecast => unimplemented!(),
                            AlonzoUtxoPredFailure::TooManyCollateralInputs => unimplemented!(),
                            AlonzoUtxoPredFailure::NoCollateralInputs => unimplemented!(),
                        }
                    }
                    BabbageUtxoPredFailure::IncorrectTotalCollateralField => unimplemented!(),
                    BabbageUtxoPredFailure::BabbageOutputTooSmallUTxO => unimplemented!(),
                    BabbageUtxoPredFailure::BabbageNonDisjointRefInputs => unimplemented!(),
                },
                BabbageUtxowPredFailure::MalformedScriptWitnesses => unimplemented!(),
                BabbageUtxowPredFailure::MalformedReferenceScripts => unimplemented!(),
            },
            ShelleyLedgerPredFailure::DelegsFailure => unimplemented!(),
        }
    }
}
