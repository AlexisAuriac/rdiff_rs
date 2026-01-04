use std::io::{self, Read, Seek, SeekFrom, Write, copy};

use crate::{
    delta::builder::DELTA_MAGIC,
    error::Error,
    op::{OP2CMD, OpArgLen, OpKind},
};

fn copy_n<R, W>(reader: &mut R, writer: &mut W, len: u64) -> Result<(), io::Error>
where
    R: Read,
    W: Write,
{
    let n = copy(&mut reader.take(len), writer)?;
    if n != len {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "failed to fill whole buffer",
        ));
    }

    Ok(())
}

fn read_param<I>(i: &mut I, size: OpArgLen) -> Result<u64, Error>
where
    I: Read,
{
    match size {
        OpArgLen::N1 => {
            let mut buf = [0u8; 1];
            i.read_exact(&mut buf)?;
            Ok(buf[0] as u64)
        }
        OpArgLen::N2 => {
            let mut buf = [0u8; 2];
            i.read_exact(&mut buf)?;
            Ok(u16::from_be_bytes(buf) as u64)
        }
        OpArgLen::N4 => {
            let mut buf = [0u8; 4];
            i.read_exact(&mut buf)?;
            Ok(u32::from_be_bytes(buf) as u64)
        }
        OpArgLen::N8 => {
            let mut buf = [0u8; 8];
            i.read_exact(&mut buf)?;
            Ok(u64::from_be_bytes(buf))
        }
    }
}

pub fn patch<I, D, O>(old: &mut I, delta: &mut D, out: &mut O) -> Result<(), Error>
where
    I: Read + Seek,
    D: Read,
    O: Write,
{
    let mut magic_buf = [0u8; 4];
    delta.read_exact(&mut magic_buf)?;
    let magic = u32::from_be_bytes(magic_buf);
    if magic != DELTA_MAGIC {
        return Err(Error::BadMagic {
            expect_name: "delta".to_string(),
            expect_value: DELTA_MAGIC,
            got: magic,
        });
    }

    loop {
        let mut op_buf = [0u8];
        delta.read_exact(&mut op_buf)?;
        let op = op_buf[0];
        let cmd = &OP2CMD[op as usize];

        let (param1, param2) = match (cmd.len1, cmd.len2) {
            (None, _) => (cmd.immediate as u64, 0),
            (Some(len1), None) => (read_param(delta, len1)?, 0),
            (Some(len1), Some(len2)) => (read_param(delta, len1)?, read_param(delta, len2)?),
        };

        match cmd.kind {
            OpKind::Literal => {
                let len = param1;

                copy_n(delta, out, len)?;
            }
            OpKind::Copy => {
                let pos = param1;
                let len = param2;

                old.seek(SeekFrom::Start(pos))?;
                copy_n(old, out, len)?;
            }
            OpKind::End => break,
            _ => return Err(Error::UnexpectedCommand(cmd.kind)),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::{self, Cursor},
        path::PathBuf,
    };

    use crate::{delta::delta, op::Op, signature::read_signature_file, strong_sum::StrongType};

    use super::*;

    #[derive(Debug, Clone)]
    enum DeltaElem {
        Op(Op),
        U8(u8),
        U16(u16),
        U32(u32),
        U64(u64),
        Arr(Vec<u8>),
    }

    impl From<Op> for DeltaElem {
        fn from(val: Op) -> Self {
            DeltaElem::Op(val)
        }
    }

    impl From<u8> for DeltaElem {
        fn from(val: u8) -> Self {
            DeltaElem::U8(val)
        }
    }

    impl From<u16> for DeltaElem {
        fn from(val: u16) -> Self {
            DeltaElem::U16(val)
        }
    }

    impl From<u32> for DeltaElem {
        fn from(val: u32) -> Self {
            DeltaElem::U32(val)
        }
    }

    impl From<u64> for DeltaElem {
        fn from(val: u64) -> Self {
            DeltaElem::U64(val)
        }
    }

    impl<const T: usize> From<[u8; T]> for DeltaElem {
        fn from(val: [u8; T]) -> Self {
            DeltaElem::Arr(val.to_vec())
        }
    }

    impl DeltaElem {
        pub fn into_vec(self) -> Vec<u8> {
            match self {
                DeltaElem::Op(op) => vec![op as u8],
                DeltaElem::U8(n) => vec![n],
                DeltaElem::U16(n) => n.to_be_bytes().to_vec(),
                DeltaElem::U32(n) => n.to_be_bytes().to_vec(),
                DeltaElem::U64(n) => n.to_be_bytes().to_vec(),
                DeltaElem::Arr(bytes) => bytes,
            }
        }
    }

    fn make_delta(elems: Vec<DeltaElem>) -> Vec<u8> {
        elems.into_iter().flat_map(|e| e.into_vec()).collect()
    }

    fn generic_patch_err_test(
        delta: Vec<u8>,
        old: Option<Vec<u8>>,
        new: Option<&mut Vec<u8>>,
    ) -> Error {
        let old = old.unwrap_or(vec![]);
        let mut new_buf = vec![];
        let new = new.unwrap_or(&mut new_buf);

        let Err(err) = patch(
            &mut Cursor::new(old),
            &mut Cursor::new(delta),
            &mut Cursor::new(new),
        ) else {
            panic!("got no error")
        };

        err
    }

    #[test]
    fn err_empty() {
        let delta = make_delta(vec![]);

        match generic_patch_err_test(delta, None, None) {
            Error::Io(err) if err.kind() == io::ErrorKind::UnexpectedEof => (),
            err => panic!("expected UnexpectedEof, got {err:?}"),
        }
    }

    #[test]
    fn err_bad_magic() {
        let bad_magic = DELTA_MAGIC.rotate_right(1);
        let delta = make_delta(vec![bad_magic.into()]);

        match generic_patch_err_test(delta, None, None) {
            Error::BadMagic {
                expect_name,
                expect_value,
                got,
            } if expect_name == "delta" && expect_value == DELTA_MAGIC && got == bad_magic => {}
            err => panic!(
                "expected {:?}, got {err:?}",
                Error::BadMagic {
                    expect_name: "delta".to_string(),
                    expect_value: DELTA_MAGIC,
                    got: bad_magic,
                },
            ),
        }
    }

    #[test]
    fn err_no_end_op() {
        let delta = make_delta(vec![DELTA_MAGIC.into()]);

        match generic_patch_err_test(delta, None, None) {
            Error::Io(err) if err.kind() == io::ErrorKind::UnexpectedEof => (),
            err => panic!("expected UnexpectedEof, got {err:?}"),
        }
    }

    #[test]
    fn err_unexpected_command() {
        let delta = make_delta(vec![DELTA_MAGIC.into(), Op::Reserved85.into()]);

        match generic_patch_err_test(delta, None, None) {
            Error::UnexpectedCommand(OpKind::Reserved) => (),
            err => panic!(
                "expected {:?}, got {err:?}",
                Error::UnexpectedCommand(OpKind::Reserved),
            ),
        }
    }

    #[test]
    fn err_literal_read_missing_len() {
        let delta = make_delta(vec![DELTA_MAGIC.into(), Op::LiteralN1.into()]);

        match generic_patch_err_test(delta, None, None) {
            Error::Io(err) if err.kind() == io::ErrorKind::UnexpectedEof => (),
            err => panic!("expected UnexpectedEof, got {err:?}"),
        }
    }

    #[test]
    fn err_literal_read_partial_len() {
        let delta = make_delta(vec![DELTA_MAGIC.into(), Op::LiteralN8.into(), 1u32.into()]);

        match generic_patch_err_test(delta, None, None) {
            Error::Io(err) if err.kind() == io::ErrorKind::UnexpectedEof => (),
            err => panic!("expected UnexpectedEof, got {err:?}"),
        }
    }

    #[test]
    fn err_literal_invalid_len() {
        let delta = make_delta(vec![DELTA_MAGIC.into(), Op::LiteralN1.into(), 1u8.into()]);

        match generic_patch_err_test(delta, None, None) {
            Error::Io(err) if err.kind() == io::ErrorKind::UnexpectedEof => (),
            err => panic!("expected UnexpectedEof, got {err:?}"),
        }
    }

    #[test]
    fn err_copy_missing_pos() {
        let delta = make_delta(vec![DELTA_MAGIC.into(), Op::CopyN1N1.into()]);

        match generic_patch_err_test(delta, None, None) {
            Error::Io(err) if err.kind() == io::ErrorKind::UnexpectedEof => (),
            err => panic!("expected UnexpectedEof, got {err:?}"),
        }
    }

    #[test]
    fn err_copy_partial_pos() {
        let delta = make_delta(vec![DELTA_MAGIC.into(), Op::CopyN8N1.into(), 1u32.into()]);

        match generic_patch_err_test(delta, None, None) {
            Error::Io(err) if err.kind() == io::ErrorKind::UnexpectedEof => (),
            err => panic!("expected UnexpectedEof, got {err:?}"),
        }
    }

    #[test]
    fn err_copy_missing_len() {
        let delta = make_delta(vec![DELTA_MAGIC.into(), Op::CopyN1N1.into(), 1u8.into()]);

        match generic_patch_err_test(delta, None, None) {
            Error::Io(err) if err.kind() == io::ErrorKind::UnexpectedEof => (),
            err => panic!("expected UnexpectedEof, got {err:?}"),
        }
    }

    #[test]
    fn err_copy_partial_len() {
        let delta = make_delta(vec![
            DELTA_MAGIC.into(),
            Op::CopyN1N8.into(),
            1u8.into(),
            1u32.into(),
        ]);

        match generic_patch_err_test(delta, None, None) {
            Error::Io(err) if err.kind() == io::ErrorKind::UnexpectedEof => (),
            err => panic!("expected UnexpectedEof, got {err:?}"),
        }
    }

    #[test]
    fn err_copy_pos_out_of_bounds() {
        let delta = make_delta(vec![
            DELTA_MAGIC.into(),
            Op::CopyN1N1.into(),
            100u8.into(),
            1u8.into(),
            Op::EndOp.into(),
        ]);

        match generic_patch_err_test(delta, None, None) {
            Error::Io(err) if err.kind() == io::ErrorKind::UnexpectedEof => (),
            err => panic!("expected UnexpectedEof, got {err:?}"),
        }
    }

    #[test]
    fn err_copy_len_out_of_bounds() {
        let delta = make_delta(vec![
            DELTA_MAGIC.into(),
            Op::CopyN1N8.into(),
            0u8.into(),
            1u8.into(),
            Op::EndOp.into(),
        ]);

        match generic_patch_err_test(delta, None, None) {
            Error::Io(err) if err.kind() == io::ErrorKind::UnexpectedEof => (),
            err => panic!("expected UnexpectedEof, got {err:?}"),
        }
    }

    fn generic_patch_test(
        name: &str,
        strong_type: &str,
        block_len: u32,
        strong_len: u32,
    ) -> Result<(), Error> {
        strong_type.parse::<StrongType>()?;
        let file_base_name = format!("{}-{}-{}-{}", name, strong_type, block_len, strong_len);

        let old_path = PathBuf::from("testdata").join(name).with_extension("old");
        let mut old_data = Cursor::new(fs::read(old_path)?);

        let delta_path = PathBuf::from("testdata")
            .join(&file_base_name)
            .with_extension("delta");
        let mut delta_data = Cursor::new(fs::read(delta_path)?);

        let mut output = Cursor::new(vec![]);
        patch(&mut old_data, &mut delta_data, &mut output)?;

        let want_new_path = PathBuf::from("testdata").join(name).with_extension("new");
        let want_new_data = fs::read(want_new_path)?;

        assert_eq!(output.into_inner(), want_new_data);

        Ok(())
    }

    macro_rules! test_patch {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() -> Result<(), Error> {
                    let (name, strong_type, block_len, strong_len) = $value;
                    generic_patch_test(name, strong_type, block_len, strong_len)
                }
            )*
        };
    }

    test_patch!(
        patch_000_blake2_11_23: ("000", "blake2", 11, 23),
        patch_000_blake2_512_32: ("000", "blake2", 512, 32),
        patch_000_md4_256_7: ("000", "md4", 256, 7),
        patch_001_blake2_512_32: ("001", "blake2", 512, 32),
        patch_001_blake2_776_31: ("001", "blake2", 776, 31),
        patch_001_md4_777_15: ("001", "md4", 777, 15),
        patch_002_blake2_512_32: ("002", "blake2", 512, 32),
        patch_002_blake2_431_19: ("002", "blake2", 431, 19),
        patch_002_md4_128_16: ("002", "md4", 128, 16),
        patch_003_blake2_512_32: ("003", "blake2", 512, 32),
        patch_003_blake2_1024_13: ("003", "blake2", 1024, 13),
        patch_003_md4_1024_13: ("003", "md4", 1024, 13),
        patch_004_blake2_1024_28: ("004", "blake2", 1024, 28),
        patch_004_blake2_2222_31: ("004", "blake2", 2222, 31),
        patch_004_blake2_512_32: ("004", "blake2", 512, 32),
        patch_005_blake2_512_32: ("005", "blake2", 512, 32),
        patch_005_blake2_1000_18: ("005", "blake2", 1000, 18),
        patch_005_md4_999_14: ("005", "md4", 999, 14),
        patch_006_blake2_2_32: ("006", "blake2", 2, 32),
        patch_007_blake2_5_32: ("007", "blake2", 5, 32),
        patch_007_blake2_4_32: ("007", "blake2", 4, 32),
        patch_007_blake2_3_32: ("007", "blake2", 3, 32),
        patch_008_blake2_222_30: ("008", "blake2", 222, 30),
        patch_008_blake2_512_32: ("008", "blake2", 512, 32),
        patch_008_md4_111_11: ("008", "md4", 111, 11),
        patch_009_blake2_2048_26: ("009", "blake2", 2048, 26),
        patch_009_blake2_512_32: ("009", "blake2", 512, 32),
        patch_009_md4_2033_15: ("009", "md4", 2033, 15),
        patch_010_blake2_512_32: ("010", "blake2", 512, 32),
        patch_010_blake2_7_6: ("010", "blake2", 7, 6),
        patch_010_md4_4096_8: ("010", "md4", 4096, 8),
        patch_011_blake2_3_32: ("011", "blake2", 3, 32),
        patch_011_md4_3_9: ("011", "md4", 3, 9),
    );

    fn generic_delta_and_patch_test(
        name: &str,
        strong_type: &str,
        block_len: u32,
        strong_len: u32,
    ) -> Result<(), Error> {
        strong_type.parse::<StrongType>()?;
        let file_base_name = format!("{}-{}-{}-{}", name, strong_type, block_len, strong_len);

        let sig_path = PathBuf::from("testdata")
            .join(file_base_name)
            .with_extension("signature");
        let sig = read_signature_file(&sig_path)?;

        let new_path = PathBuf::from("testdata").join(name).with_extension("new");
        let mut new_data = Cursor::new(fs::read(new_path)?);

        let mut delta_data = Cursor::new(vec![]);
        delta(&sig, &mut new_data, &mut delta_data)?;

        let old_path = PathBuf::from("testdata").join(name).with_extension("old");
        let mut old_data = Cursor::new(fs::read(old_path)?);

        delta_data.set_position(0);
        let mut patched_data = Cursor::new(vec![]);
        patch(&mut old_data, &mut delta_data, &mut patched_data)?;

        assert_eq!(patched_data.into_inner(), new_data.into_inner());

        Ok(())
    }

    macro_rules! test_delta_and_patch {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() -> Result<(), Error> {
                    let (name, strong_type, block_len, strong_len) = $value;
                    generic_delta_and_patch_test(name, strong_type, block_len, strong_len)
                }
            )*
        };
    }

    test_delta_and_patch!(
        delta_and_patch_000_blake2_11_23: ("000", "blake2", 11, 23),
        delta_and_patch_000_blake2_512_32: ("000", "blake2", 512, 32),
        delta_and_patch_000_md4_256_7: ("000", "md4", 256, 7),
        delta_and_patch_001_blake2_512_32: ("001", "blake2", 512, 32),
        delta_and_patch_001_blake2_776_31: ("001", "blake2", 776, 31),
        delta_and_patch_001_md4_777_15: ("001", "md4", 777, 15),
        delta_and_patch_002_blake2_512_32: ("002", "blake2", 512, 32),
        delta_and_patch_002_blake2_431_19: ("002", "blake2", 431, 19),
        delta_and_patch_002_md4_128_16: ("002", "md4", 128, 16),
        delta_and_patch_003_blake2_512_32: ("003", "blake2", 512, 32),
        delta_and_patch_003_blake2_1024_13: ("003", "blake2", 1024, 13),
        delta_and_patch_003_md4_1024_13: ("003", "md4", 1024, 13),
        delta_and_patch_004_blake2_1024_28: ("004", "blake2", 1024, 28),
        delta_and_patch_004_blake2_2222_31: ("004", "blake2", 2222, 31),
        delta_and_patch_004_blake2_512_32: ("004", "blake2", 512, 32),
        delta_and_patch_005_blake2_512_32: ("005", "blake2", 512, 32),
        delta_and_patch_005_blake2_1000_18: ("005", "blake2", 1000, 18),
        delta_and_patch_005_md4_999_14: ("005", "md4", 999, 14),
        delta_and_patch_006_blake2_2_32: ("006", "blake2", 2, 32),
        delta_and_patch_007_blake2_5_32: ("007", "blake2", 5, 32),
        delta_and_patch_007_blake2_4_32: ("007", "blake2", 4, 32),
        delta_and_patch_007_blake2_3_32: ("007", "blake2", 3, 32),
        delta_and_patch_008_blake2_222_30: ("008", "blake2", 222, 30),
        delta_and_patch_008_blake2_512_32: ("008", "blake2", 512, 32),
        delta_and_patch_008_md4_111_11: ("008", "md4", 111, 11),
        delta_and_patch_009_blake2_2048_26: ("009", "blake2", 2048, 26),
        delta_and_patch_009_blake2_512_32: ("009", "blake2", 512, 32),
        delta_and_patch_009_md4_2033_15: ("009", "md4", 2033, 15),
        delta_and_patch_010_blake2_512_32: ("010", "blake2", 512, 32),
        delta_and_patch_010_blake2_7_6: ("010", "blake2", 7, 6),
        delta_and_patch_010_md4_4096_8: ("010", "md4", 4096, 8),
        delta_and_patch_011_blake2_3_32: ("011", "blake2", 3, 32),
        delta_and_patch_011_md4_3_9: ("011", "md4", 3, 9),
    );
}
