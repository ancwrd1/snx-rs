use std::fmt;

use anyhow::anyhow;
use bytes::{Buf, BufMut, BytesMut};
use serde::Serialize;
use tokio_util::codec::{Decoder, Encoder};

use crate::{
    model::proto::{
        ClientHello, ClientHelloData, DisconnectRequest, DisconnectRequestData, KeepaliveRequest, KeepaliveRequestData,
    },
    sexpr::SExpression,
};

const PKT_CONTROL: u32 = 1;
const PKT_DATA: u32 = 2;

pub enum SlimPacketType {
    Control(SExpression),
    Data(Vec<u8>),
}

impl fmt::Debug for SlimPacketType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SlimPacketType::Control(expr) => write!(f, "CONTROL: {}", expr.object_name().unwrap_or("???")),
            SlimPacketType::Data(data) => write!(f, "DATA: {} bytes", data.len()),
        }
    }
}

impl SlimPacketType {
    pub fn control<T>(data: T) -> Self
    where
        T: Serialize + Default,
    {
        SlimPacketType::Control(data.into())
    }
}

impl From<Vec<u8>> for SlimPacketType {
    fn from(value: Vec<u8>) -> Self {
        SlimPacketType::Data(value)
    }
}

impl From<ClientHelloData> for SlimPacketType {
    fn from(value: ClientHelloData) -> Self {
        SlimPacketType::control(ClientHello { data: value })
    }
}

impl From<KeepaliveRequestData> for SlimPacketType {
    fn from(value: KeepaliveRequestData) -> Self {
        SlimPacketType::control(KeepaliveRequest { data: value })
    }
}

impl From<DisconnectRequestData> for SlimPacketType {
    fn from(value: DisconnectRequestData) -> Self {
        SlimPacketType::control(DisconnectRequest { data: value })
    }
}

pub(crate) struct SlimProtocolCodec;

impl Decoder for SlimProtocolCodec {
    type Item = SlimPacketType;
    type Error = anyhow::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.remaining() < 4 {
            return Ok(None);
        }

        let len = u32::from_be_bytes(src[0..4].try_into()?) as usize;

        if src.remaining() < 8 + len {
            return Ok(None);
        }

        let packet_type = u32::from_be_bytes(src[4..8].try_into()?);

        match packet_type {
            PKT_CONTROL => {
                let s_data = String::from_utf8_lossy(&src[8..8 + len]).into_owned();
                src.advance(8 + len);
                Ok(Some(SlimPacketType::Control(s_data.parse()?)))
            }
            PKT_DATA => {
                let data = src[8..8 + len].to_vec();
                src.advance(8 + len);
                Ok(Some(SlimPacketType::Data(data)))
            }
            _ => Err(anyhow!(i18n::tr!("error-unknown-packet-type"))),
        }
    }
}

impl Encoder<SlimPacketType> for SlimProtocolCodec {
    type Error = anyhow::Error;

    fn encode(&mut self, item: SlimPacketType, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let (data, packet_type) = match item {
            SlimPacketType::Control(expr) => {
                let mut data = expr.to_string().into_bytes();
                data.push(b'\x00');
                (data, PKT_CONTROL)
            }
            SlimPacketType::Data(data) => (data, PKT_DATA),
        };

        dst.reserve(data.len() + 8);

        let data_len = (data.len() as u32).to_be_bytes();
        let packet_type = packet_type.to_be_bytes();

        dst.put_slice(&data_len);
        dst.put_slice(&packet_type);
        dst.put_slice(&data);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keepalive() -> SlimPacketType {
        SlimPacketType::from(KeepaliveRequestData { id: "42".to_owned() })
    }

    #[test]
    fn encode_data_packet() {
        let mut dst = BytesMut::new();
        SlimProtocolCodec
            .encode(SlimPacketType::Data(vec![1, 2, 3, 4, 5]), &mut dst)
            .unwrap();

        assert_eq!(&dst[..], &[0, 0, 0, 5, 0, 0, 0, 2, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn encode_control_packet_is_nul_terminated() {
        let packet = keepalive();
        let SlimPacketType::Control(ref expr) = packet else {
            unreachable!()
        };
        let expected = expr.to_string();

        let mut dst = BytesMut::new();
        SlimProtocolCodec.encode(packet, &mut dst).unwrap();

        assert_eq!(
            u32::from_be_bytes(dst[0..4].try_into().unwrap()) as usize,
            expected.len() + 1
        );
        assert_eq!(u32::from_be_bytes(dst[4..8].try_into().unwrap()), PKT_CONTROL);
        assert_eq!(&dst[8..dst.len() - 1], expected.as_bytes());
        assert_eq!(dst[dst.len() - 1], 0);
    }

    #[test]
    fn encode_appends_without_clearing() {
        let mut dst = BytesMut::new();
        let mut codec = SlimProtocolCodec;
        codec.encode(SlimPacketType::Data(vec![0xaa]), &mut dst).unwrap();
        codec.encode(SlimPacketType::Data(vec![0xbb, 0xcc]), &mut dst).unwrap();

        assert_eq!(
            &dst[..],
            &[0, 0, 0, 1, 0, 0, 0, 2, 0xaa, 0, 0, 0, 2, 0, 0, 0, 2, 0xbb, 0xcc]
        );
    }

    #[test]
    fn decode_data_roundtrip() {
        let data = vec![9, 8, 7, 6];
        let mut buf = BytesMut::new();
        let mut codec = SlimProtocolCodec;
        codec.encode(SlimPacketType::Data(data.clone()), &mut buf).unwrap();

        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        assert!(matches!(decoded, SlimPacketType::Data(d) if d == data));
        assert!(buf.is_empty());
    }

    #[test]
    fn decode_control_roundtrip_ignores_trailing_nul() {
        let packet = keepalive();
        let SlimPacketType::Control(ref expr) = packet else {
            unreachable!()
        };
        let expected = expr.clone();

        let mut buf = BytesMut::new();
        let mut codec = SlimProtocolCodec;
        codec.encode(packet, &mut buf).unwrap();

        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        assert!(matches!(decoded, SlimPacketType::Control(e) if e == expected));
        assert!(buf.is_empty());
    }

    #[test]
    fn decode_returns_none_on_partial_input() {
        let mut codec = SlimProtocolCodec;

        let mut buf = BytesMut::from(&[0, 0, 0][..]);
        assert!(codec.decode(&mut buf).unwrap().is_none());

        let mut buf = BytesMut::from(&[0, 0, 0, 4, 0, 0, 0, 2, 1, 2, 3][..]);
        assert!(codec.decode(&mut buf).unwrap().is_none());
        assert_eq!(buf.len(), 11);
    }

    #[test]
    fn decode_leaves_trailing_bytes_of_next_packet() {
        let mut buf = BytesMut::from(&[0, 0, 0, 1, 0, 0, 0, 2, 0xaa, 0, 0, 0, 1][..]);
        let mut codec = SlimProtocolCodec;

        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        assert!(matches!(decoded, SlimPacketType::Data(d) if d == vec![0xaa]));
        assert_eq!(&buf[..], &[0, 0, 0, 1]);
        assert!(codec.decode(&mut buf).unwrap().is_none());
    }

    #[test]
    fn decode_rejects_unknown_packet_type() {
        let mut buf = BytesMut::from(&[0, 0, 0, 1, 0, 0, 0, 7, 0xaa][..]);

        assert!(SlimProtocolCodec.decode(&mut buf).is_err());
    }
}
