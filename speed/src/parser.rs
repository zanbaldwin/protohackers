use crate::error::Error;
use crate::{
    models::io::ClientInput, ByteString, MESSAGE_TYPE_AM_CAMERA, MESSAGE_TYPE_AM_DISPATCHER, MESSAGE_TYPE_PLATE,
    MESSAGE_TYPE_WANT_HEARTBEAT,
};
use nom::{
    branch::alt,
    bytes::streaming::tag,
    combinator::map,
    multi::length_count,
    number::streaming::{be_u16, be_u32, be_u8},
    sequence::tuple,
    IResult,
};

fn nom_u16_arr(input: &[u8]) -> IResult<&[u8], Vec<u16>> {
    length_count(be_u8, be_u16)(input)
}

fn nom_str(input: &[u8]) -> IResult<&[u8], ByteString> {
    length_count(be_u8, be_u8)(input)
}

fn nom_plate(input: &[u8]) -> IResult<&[u8], ClientInput> {
    map(tuple((tag([MESSAGE_TYPE_PLATE]), nom_str, be_u32)), |(_, plate, timestamp)| {
        ClientInput::Plate(plate, timestamp)
    })(input)
}

fn nom_heartbeat(input: &[u8]) -> IResult<&[u8], ClientInput> {
    map(tuple((tag([MESSAGE_TYPE_WANT_HEARTBEAT]), be_u32)), |(_, interval)| {
        ClientInput::WantHeartbeat(interval)
    })(input)
}

fn nom_camera(input: &[u8]) -> IResult<&[u8], ClientInput> {
    map(tuple((tag([MESSAGE_TYPE_AM_CAMERA]), be_u16, be_u16, be_u16)), |(_, road, mile, limit)| {
        ClientInput::IAmCamera(road, mile, limit)
    })(input)
}

fn nom_dispatcher(input: &[u8]) -> IResult<&[u8], ClientInput> {
    map(tuple((tag([MESSAGE_TYPE_AM_DISPATCHER]), nom_u16_arr)), |(_, roads)| {
        ClientInput::IAmDispatcher(roads)
    })(input)
}

type InputBufferMatch = Result<Option<(ClientInput, usize)>, Error>;
pub(crate) fn nom(input: &[u8]) -> InputBufferMatch {
    match alt((nom_plate, nom_camera, nom_dispatcher, nom_heartbeat))(input) {
        Ok((remainder, client_message)) => Ok(Some((client_message, input.len() - remainder.len()))),
        Err(err) => match err {
            nom::Err::Incomplete(_) => Ok(None),
            _ => Err(Error::InvalidData),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::nom;
    use crate::error::Error;
    use crate::models::io::ClientInput;

    #[test]
    fn test_invalid() {
        assert_eq!(Error::InvalidData, nom(&[0u8]).err().expect("Parsing should have failed."));
    }

    #[test]
    fn test_camera_incomplete() {
        assert!(nom(&[0x80u8, 0x03, 0x11, 0x0c, 0x9d])
            .ok()
            .expect("Parser should not have failed, just incomplete")
            .is_none());
    }

    #[test]
    fn test_camera_exact() {
        assert_eq!(
            Ok(Some((ClientInput::IAmCamera(785, 3229, 100), 7))),
            nom(&[0x80u8, 0x03, 0x11, 0x0c, 0x9d, 0x00, 0x64])
        );
    }

    #[test]
    fn test_camera_overflow() {
        assert_eq!(
            Ok(Some((ClientInput::IAmCamera(785, 3229, 100), 7))),
            nom(&[0x80u8, 0x03, 0x11, 0x0c, 0x9d, 0x00, 0x64, 0x12, 0x34, 0x56])
        );
    }

    #[test]
    fn test_dispatcher_incomplete() {
        assert_eq!(Ok(None), nom(&[0x81u8, 0x01, 0x03]));
        assert_eq!(Ok(None), nom(&[0x81u8, 0x03, 0x0c, 0x9d, 0x00, 0x64, 0x12]));
    }

    #[test]
    fn test_dispatcher_exact() {
        assert_eq!(Ok(Some((ClientInput::IAmDispatcher(vec![785]), 4))), nom(&[0x81u8, 0x01, 0x03, 0x11]));
    }

    #[test]
    fn test_dispatcher_single_overflow() {
        assert_eq!(Ok(Some((ClientInput::IAmDispatcher(vec![785]), 4))), nom(&[0x81u8, 0x01, 0x03, 0x11, 0xab]));
    }

    #[test]
    fn test_dispatcher_multiple_overflow() {
        assert_eq!(
            Ok(Some((ClientInput::IAmDispatcher(vec![785, 43933]), 6))),
            nom(&[0x81u8, 0x02, 0x03, 0x11, 0xab, 0x9d, 0x00, 0x64, 0x12])
        );
    }
}
