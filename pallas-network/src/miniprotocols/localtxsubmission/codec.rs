use std::fmt::{Debug, Display};

use pallas_codec::minicbor::data::Tag;
use pallas_codec::minicbor::{decode, encode, Decode, Decoder, Encode, Encoder};

use crate::miniprotocols::localtxsubmission::{EraTx, Message, RejectReason};

impl<Tx, Reject> Encode<()> for Message<Tx, Reject>
where
    Tx: Encode<()>,
    Reject: Encode<()>,
{
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            Message::SubmitTx(tx) => {
                e.array(2)?.u16(0)?;
                e.encode(tx)?;
                Ok(())
            }
            Message::AcceptTx => {
                e.array(1)?.u16(1)?;
                Ok(())
            }
            Message::RejectTx(rejection) => {
                e.array(2)?.u16(2)?;
                e.encode(rejection)?;
                Ok(())
            }
            Message::Done => {
                e.array(1)?.u16(3)?;
                Ok(())
            }
        }
    }
}

pub enum DecodingResult<Reject> {
    Complete(Vec<Reject>),
    Incomplete(Vec<Reject>),
}

#[derive(Debug)]
struct IncompleteDecoding<Reject> {
    pub processed_reasons: Vec<Reject>,
}

impl<Reject: Display + Debug> Display for IncompleteDecoding<Reject> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.processed_reasons)
    }
}

impl<Reject: Display + Debug> std::error::Error for IncompleteDecoding<Reject> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl<Reject> From<Vec<Reject>> for IncompleteDecoding<Reject> {
    fn from(value: Vec<Reject>) -> Self {
        IncompleteDecoding {
            processed_reasons: value,
        }
    }
}

/// An implementor of this trait is able to decode an entity from CBOR with bytes that are split
/// over multiple payloads.
pub trait DecodeCBORSplitPayload {
    /// Type of entity to decode
    type Entity;
    /// Attempt to decode entity given a new slice of bytes.
    fn try_decode_with_new_bytes(&mut self, bytes: &[u8]) -> DecodingResult<Self::Entity>;
    /// Returns true if there still remain CBOR bytes to be decoded.
    fn has_undecoded_bytes(&self) -> bool;
}

impl<'b, C, Tx: Decode<'b, ()>, Reject> Decode<'b, C> for Message<Tx, Reject>
where
    C: DecodeCBORSplitPayload<Entity = Reject>,
    Reject: Debug + Display + Send + Sync + 'static,
{
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        if ctx.has_undecoded_bytes() {
            let s = ctx.try_decode_with_new_bytes(d.input());
            match s {
                DecodingResult::Complete(reasons) => Ok(Message::RejectTx(reasons)),
                DecodingResult::Incomplete(reasons) => {
                    Err(decode::Error::custom(IncompleteDecoding::from(reasons)))
                }
            }
        } else {
            let mut probe = d.probe();
            if probe.array().is_err() {
                // If we don't have any unprocessed bytes the first element should be an array
                return Err(decode::Error::message(
                    "Expecting an array (no unprocessed bytes)",
                ));
            }
            let label = probe.u16()?;
            match label {
                0 => {
                    d.array()?;
                    d.u16()?;
                    let tx = d.decode()?;
                    Ok(Message::SubmitTx(tx))
                }
                1 => Ok(Message::AcceptTx),
                2 => {
                    let s = ctx.try_decode_with_new_bytes(d.input());
                    match s {
                        DecodingResult::Complete(reasons) => Ok(Message::RejectTx(reasons)),
                        DecodingResult::Incomplete(reasons) => {
                            Err(decode::Error::custom(IncompleteDecoding::from(reasons)))
                        }
                    }
                }
                3 => Ok(Message::Done),
                _ => Err(decode::Error::message("can't decode Message")),
            }
        }
    }
}

impl<'b> Decode<'b, ()> for EraTx {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let era = d.u16()?;
        let tag = d.tag()?;
        if tag != Tag::Cbor {
            return Err(decode::Error::message("Expected encoded CBOR data item"));
        }
        Ok(EraTx(era, d.bytes()?.to_vec()))
    }
}

impl Encode<()> for EraTx {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        e.array(2)?;
        e.u16(self.0)?;
        e.tag(Tag::Cbor)?;
        e.bytes(&self.1)?;
        Ok(())
    }
}

impl<'b> Decode<'b, ()> for RejectReason {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let remainder = d.input().to_vec();
        Ok(RejectReason(remainder))
    }
}

impl Encode<()> for RejectReason {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        e.writer_mut()
            .write_all(&self.0)
            .map_err(encode::Error::write)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use pallas_codec::minicbor::Decode;
    use pallas_codec::{minicbor, Fragment};

    use crate::miniprotocols::localtxsubmission::{EraTx, Message, RejectReason};
    use crate::multiplexer::Error;

    use super::{DecodeCBORSplitPayload, DecodingResult};

    #[test]
    fn decode_reject_message() {
        let bytes = hex::decode(RAW_REJECT_RESPONSE).unwrap();
        let mut decoder = minicbor::Decoder::new(&bytes);
        let maybe_msg: Message<EraTx, RejectReason> =
            decoder.decode_with(&mut CBORDecoder).unwrap();
        // let msg_res = try_decode_message::<Message<EraTx, RejectReason>>(&mut bytes);
        // assert!(msg_res.is_ok())
    }

    struct CBORDecoder;

    impl DecodeCBORSplitPayload for CBORDecoder {
        type Entity = RejectReason;

        fn try_decode_with_new_bytes(&mut self, bytes: &[u8]) -> DecodingResult<Self::Entity> {
            let mut decoder = minicbor::Decoder::new(bytes);
            let reason = RejectReason::decode(&mut decoder, &mut ());
            match reason {
                Ok(reason) => DecodingResult::Complete(vec![reason]),
                Err(e) => DecodingResult::Incomplete(vec![]),
            }
        }

        fn has_undecoded_bytes(&self) -> bool {
            false
        }
    }

    const RAW_REJECT_RESPONSE: &str =
        "82028182059f820082018200820a81581c3b890fb5449baedf5342a48ee9c9ec6acbc995641be92ad21f08c686\
        8200820183038158202628ce6ff8cc7ff0922072d930e4a693c17f991748dedece0be64819a2f9ef7782582031d\
        54ce8d7e8cb262fc891282f44e9d24c3902dc38fac63fd469e8bf3006376b5820750852fdaf0f2dd724291ce007\
        b8e76d74bcf28076ed0c494cd90c0cfe1c9ca582008201820782000000018200820183048158201a547638b4cf4\
        a3cec386e2f898ac6bc987fadd04277e1d3c8dab5c505a5674e8158201457e4107607f83a80c3c4ffeb70910c2b\
        a3a35cf1699a2a7375f50fcc54a931820082028201830500821a00636185a2581c6f1a1f0c7ccf632cc9ff4b796\
        87ed13ffe5b624cce288b364ebdce50a144414749581b000000032a9f8800581c795ecedb09821cb922c13060c8\
        f6377c3344fa7692551e865d86ac5da158205399c766fb7c494cddb2f7ae53cc01285474388757bc05bd575c14a\
        713a432a901820082028201820085825820497fe6401e25733c073c01164c7f2a1a05de8c95e36580f9d1b05123\
        70040def028258207911ba2b7d91ac56b05ea351282589fe30f4717a707a1b9defaf282afe5ba44200825820791\
        1ba2b7d91ac56b05ea351282589fe30f4717a707a1b9defaf282afe5ba44201825820869bcb6f35e6b7912c25e5\
        cb33fb9906b097980a83f2b8ef40b51c4ef52eccd402825820efc267ad2c15c34a117535eecc877241ed836eb3e\
        643ec90de21ca1b12fd79c20282008202820181148200820283023a000f0f6d1a004944ce820082028201830d3a\
        000f0f6d1a00106253820082028201830182811a02409e10811a024138c01a0255e528ff";
}
