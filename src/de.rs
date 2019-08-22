use crate::error::Error;

macro_rules! arr4 {
    ($s:ident) => {
        [$s[0], $s[1], $s[2], $s[3]]
    };
}

macro_rules! arr8 {
    ($s:ident) => {
        [$s[0], $s[1], $s[2], $s[3], $s[4], $s[5], $s[6], $s[7]]
    };
}

pub trait XDRIn: Sized {
    fn read_xdr(buffer: &[u8]) -> Result<(Self, u64), Error>;
}

impl XDRIn for () {
    fn read_xdr(_buffer: &[u8]) -> Result<(Self, u64), Error> {
        Ok(((), 0))
    }
}

impl XDRIn for bool {
    fn read_xdr(buffer: &[u8]) -> Result<(Self, u64), Error> {
        match i32::read_xdr(buffer) {
            Ok((1, 4)) => Ok((true, 4)),
            Ok((0, 4)) => Ok((false, 4)),
            _ => Err(Error::BoolBadFormat),
        }
    }
}

impl XDRIn for i32 {
    fn read_xdr(buffer: &[u8]) -> Result<(Self, u64), Error> {
        if buffer.len() < 4 {
            return Err(Error::IntegerBadFormat);
        }
        let result = i32::from_be_bytes(arr4!(buffer));
        Ok((result, 4))
    }
}

impl XDRIn for u32 {
    fn read_xdr(buffer: &[u8]) -> Result<(Self, u64), Error> {
        if buffer.len() < 4 {
            return Err(Error::UnsignedIntegerBadFormat);
        }
        let result = u32::from_be_bytes(arr4!(buffer));
        Ok((result, 4))
    }
}

impl XDRIn for i64 {
    fn read_xdr(buffer: &[u8]) -> Result<(Self, u64), Error> {
        if buffer.len() < 8 {
            return Err(Error::HyperBadFormat);
        }
        let result = i64::from_be_bytes(arr8!(buffer));
        Ok((result, 8))
    }
}

impl XDRIn for u64 {
    fn read_xdr(buffer: &[u8]) -> Result<(Self, u64), Error> {
        if buffer.len() < 8 {
            return Err(Error::UnsignedHyperBadFormat);
        }
        let result = u64::from_be_bytes(arr8!(buffer));
        Ok((result, 8))
    }
}

impl XDRIn for f32 {
    fn read_xdr(buffer: &[u8]) -> Result<(Self, u64), Error> {
        if buffer.len() < 4 {
            return Err(Error::FloatBadFormat);
        }
        let result = f32::from_bits(u32::from_be_bytes(arr4!(buffer)));
        Ok((result, 4))
    }
}

impl XDRIn for f64 {
    fn read_xdr(buffer: &[u8]) -> Result<(Self, u64), Error> {
        if buffer.len() < 8 {
            return Err(Error::DoubleBadFormat);
        }
        let result = f64::from_bits(u64::from_be_bytes(arr8!(buffer)));
        Ok((result, 8))
    }
}

impl XDRIn for String {
    fn read_xdr(buffer: &[u8]) -> Result<(Self, u64), Error> {
        let size = u32::read_xdr(buffer)?.0;
        let len = size as usize;
        let mut read: u64 = 4;
        if buffer.len() < len {
            return Err(Error::StringBadFormat);
        }
        let result = std::str::from_utf8(&buffer[4..len + 4]).unwrap();
        read += size as u64;
        Ok((result.to_string(), read + (4 - read % 4) % 4))
    }
}

impl<T> XDRIn for Vec<T>
where
    T: XDRIn,
{
    fn read_xdr(buffer: &[u8]) -> Result<(Self, u64), Error> {
        let size = u32::read_xdr(buffer)?.0;
        let mut read: u64 = 4;
        let mut result = Vec::new();
        for _ in 0..size {
            let t_read = T::read_xdr(&buffer[read as usize..])?;
            read += t_read.1;
            result.push(t_read.0);
        }
        Ok((result, read))
    }
}

impl XDRIn for Vec<u8> {
    fn read_xdr(buffer: &[u8]) -> Result<(Self, u64), Error> {
        let len = u32::read_xdr(buffer)?.0;
        let size = len as usize;
        let mut read: u64 = 4;
        let result = buffer[4..size + 4].to_vec();
        read += size as u64;
        Ok((result, read + (4 - read % 4) % 4))
    }
}

pub fn read_fixed_array<T: XDRIn>(size: u32, buffer: &[u8]) -> Result<(Vec<T>, u64), Error> {
    let mut read: u64 = 0;
    let mut result = Vec::new();
    for _ in 0..size {
        let t_res = T::read_xdr(&buffer[read as usize..])?;
        read += t_res.1;
        result.push(t_res.0);
    }
    Ok((result, read))
}

pub fn read_var_array<T: XDRIn>(size: u32, buffer: &[u8]) -> Result<(Vec<T>, u64), Error> {
    let length = u32::read_xdr(buffer)?.0;
    if length > size {
        return Err(Error::BadArraySize);
    }
    let result = read_fixed_array(length, &buffer[4..])?;
    Ok((result.0, result.1 + 4))
}

pub fn read_var_opaque(max_size: u32, buffer: &[u8]) -> Result<(Vec<u8>, u64), Error> {
    let length = u32::read_xdr(buffer)?.0;
    if length > max_size {
        return Err(Error::BadArraySize);
    }
    let result = read_fixed_opaque(length, &buffer[4..])?;
    Ok((result.0, result.1 + 4))
}

pub fn read_fixed_opaque(size: u32, buffer: &[u8]) -> Result<(Vec<u8>, u64), Error> {
    let padded_size = (4 - size % 4) % 4 + size;
    if buffer.len() < padded_size as usize {
        return Err(Error::BadArraySize);
    }
    return Ok((buffer[..size as usize].to_vec(), padded_size as u64));
}

pub fn read_var_string(max_size: u32, buffer: &[u8]) -> Result<(String, u64), Error> {
    let length = u32::read_xdr(buffer)?.0;
    if length > max_size {
        return Err(Error::VarArrayWrongSize);
    }
    String::read_xdr(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bool_true() {
        let to_des: Vec<u8> = vec![0, 0, 0, 1];
        assert_eq!((true, 4), bool::read_xdr(&to_des).unwrap());
    }

    #[test]
    fn test_bool_false() {
        let to_des: Vec<u8> = vec![0, 0, 0, 0];
        assert_eq!((false, 4), bool::read_xdr(&to_des).unwrap());
    }

    #[test]
    fn test_bool_error() {
        let err_1: Vec<u8> = vec![0, 0, 0, 2];
        let err_2: Vec<u8> = vec![0, 0, 1, 0];
        let err_3: Vec<u8> = vec![0, 0, 0];
        assert_eq!(Err(Error::BoolBadFormat), bool::read_xdr(&err_1));
        assert_eq!(Err(Error::BoolBadFormat), bool::read_xdr(&err_2));
        assert_eq!(Err(Error::BoolBadFormat), bool::read_xdr(&err_3));
    }

    #[test]
    fn test_int() {
        let to_des: Vec<u8> = vec![255, 255, 255, 255];
        assert_eq!((-1, 4), i32::read_xdr(&to_des).unwrap());
    }

    #[test]
    fn test_int_error() {
        let to_des: Vec<u8> = vec![255, 255, 255];
        assert_eq!(Err(Error::IntegerBadFormat), i32::read_xdr(&to_des));
    }

    #[test]
    fn test_uint() {
        let to_des: Vec<u8> = vec![255, 255, 255, 255];
        assert_eq!((std::u32::MAX, 4), u32::read_xdr(&to_des).unwrap());
    }

    #[test]
    fn test_uint_error() {
        let to_des: Vec<u8> = vec![255, 255, 255];
        assert_eq!(Err(Error::UnsignedIntegerBadFormat), u32::read_xdr(&to_des));
    }

    #[test]
    fn test_hyper() {
        let to_des: Vec<u8> = vec![255, 255, 255, 255, 255, 255, 255, 255];
        assert_eq!((-1, 8), i64::read_xdr(&to_des).unwrap());
    }

    #[test]
    fn test_hyper_error() {
        let to_des: Vec<u8> = vec![255, 255, 255, 255, 255, 255, 255];
        assert_eq!(Err(Error::HyperBadFormat), i64::read_xdr(&to_des));
    }

    #[test]
    fn test_uhyper() {
        let to_des: Vec<u8> = vec![255, 255, 255, 255, 255, 255, 255, 255];
        assert_eq!((std::u64::MAX, 8), u64::read_xdr(&to_des).unwrap());
    }

    #[test]
    fn test_uhyper_error() {
        let to_des: Vec<u8> = vec![255, 255, 255, 255, 255, 255, 255];
        assert_eq!(Err(Error::UnsignedHyperBadFormat), u64::read_xdr(&to_des));
    }

    #[test]
    fn test_float() {
        let to_des: Vec<u8> = vec![0x3f, 0x80, 0, 0];
        assert_eq!((1.0, 4), f32::read_xdr(&to_des).unwrap());
    }

    #[test]
    fn test_float_error() {
        let to_des: Vec<u8> = vec![255, 255, 255];
        assert_eq!(Err(Error::FloatBadFormat), f32::read_xdr(&to_des));
    }

    #[test]
    fn test_double() {
        let to_des: Vec<u8> = vec![0x3f, 0xf0, 0, 0, 0, 0, 0, 0];
        assert_eq!((1.0, 8), f64::read_xdr(&to_des).unwrap());
    }

    #[test]
    fn test_double_error() {
        let to_des: Vec<u8> = vec![255, 255, 255, 255, 255, 255, 255];
        assert_eq!(Err(Error::DoubleBadFormat), f64::read_xdr(&to_des));
    }

    #[test]
    fn test_var_opaque_no_padding() {
        let to_des: Vec<u8> = vec![0, 0, 0, 8, 3, 3, 3, 4, 1, 2, 3, 4];
        let result: (Vec<u8>, u64) = Vec::read_xdr(&to_des).unwrap();
        assert_eq!((vec![3, 3, 3, 4, 1, 2, 3, 4], 12), result);
    }

    #[test]
    fn test_var_opaque_padding() {
        let to_des: Vec<u8> = vec![0, 0, 0, 5, 3, 3, 3, 4, 1, 0, 0, 0];
        let result: (Vec<u8>, u64) = Vec::read_xdr(&to_des).unwrap();
        assert_eq!((vec![3, 3, 3, 4, 1], 12), result);
    }

    #[test]
    fn test_var_array() {
        let to_des: Vec<u8> = vec![0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 3];
        let result: (Vec<u32>, u64) = Vec::read_xdr(&to_des).unwrap();
        assert_eq!((vec![1, 3], 12), result);
    }

    #[test]
    fn test_var_array_error() {
        let to_des: Vec<u8> = vec![0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0];
        let result: Result<(Vec<u32>, u64), Error> = Vec::read_xdr(&to_des);
        assert_eq!(Err(Error::UnsignedIntegerBadFormat), result);
    }

    #[derive(XDRIn, PartialEq, Debug)]
    struct TestStruct {
        one: f32,
        two: u32,
    }

    #[test]
    fn test_struct() {
        let to_des: Vec<u8> = vec![0x3f, 0x80, 0, 0, 0, 0, 0, 2];
        let expected = TestStruct { one: 1.0, two: 2 };
        let result: (TestStruct, u64) = TestStruct::read_xdr(&to_des).unwrap();
        assert_eq!((expected, 8), result);
    }

    #[test]
    fn test_struct_error() {
        let to_des: Vec<u8> = vec![0x3f, 0x80, 0, 0, 0, 0, 0];
        let result: Result<(TestStruct, u64), Error> = TestStruct::read_xdr(&to_des);
        assert_eq!(Err(Error::UnsignedIntegerBadFormat), result);
    }

    #[test]
    fn test_string() {
        let to_des: Vec<u8> = vec![0, 0, 0, 5, 104, 101, 108, 108, 111, 0, 0, 0];
        assert_eq!(
            ("hello".to_string(), 12),
            String::read_xdr(&to_des).unwrap()
        );
    }

    #[derive(XDRIn, Debug, PartialEq)]
    struct TestStringLength {
        #[array(var = 5)]
        pub string: String,
    }

    #[test]
    fn test_string_length() {
        let to_des: Vec<u8> = vec![0, 0, 0, 5, 104, 101, 108, 108, 111, 0, 0, 0];
        let expected = TestStringLength {
            string: "hello".to_string(),
        };
        assert_eq!((expected, 12), TestStringLength::read_xdr(&to_des).unwrap());
    }

    #[test]
    fn test_string_length_error() {
        let to_des: Vec<u8> = vec![0, 0, 0, 7, 104, 101, 108, 108, 111, 0, 0, 0];
        assert_eq!(
            Err(Error::VarArrayWrongSize),
            TestStringLength::read_xdr(&to_des)
        );
    }

    #[derive(XDRIn, Debug, PartialEq)]
    enum TestEnum {
        Zero = 0,
        One = 1,
        Two = 2,
    }

    #[test]
    fn test_enum() {
        let to_des1: Vec<u8> = vec![0, 0, 0, 0];
        let to_des2: Vec<u8> = vec![0, 0, 0, 1];
        let to_des3: Vec<u8> = vec![0, 0, 0, 2];

        assert_eq!((TestEnum::Zero, 4), TestEnum::read_xdr(&to_des1).unwrap());
        assert_eq!((TestEnum::One, 4), TestEnum::read_xdr(&to_des2).unwrap());
        assert_eq!((TestEnum::Two, 4), TestEnum::read_xdr(&to_des3).unwrap());
    }

    #[test]
    fn test_enum_error() {
        let to_des1: Vec<u8> = vec![1, 0, 0, 0];
        let to_des2: Vec<u8> = vec![0, 1, 0, 1];
        let to_des3: Vec<u8> = vec![0, 0, 0, 3];

        assert_eq!(Err(Error::InvalidEnumValue), TestEnum::read_xdr(&to_des1));
        assert_eq!(Err(Error::InvalidEnumValue), TestEnum::read_xdr(&to_des2));
        assert_eq!(Err(Error::InvalidEnumValue), TestEnum::read_xdr(&to_des3));
    }

    #[derive(XDRIn, Debug, PartialEq)]
    struct TestFixedOpaqueNoPadding {
        #[array(fixed = 8)]
        pub opaque: Vec<u8>,
    }

    #[test]
    fn test_fixed_opaque_no_padding() {
        let to_des: Vec<u8> = vec![3, 3, 3, 4, 1, 2, 3, 4];
        let expected = TestFixedOpaqueNoPadding {
            opaque: vec![3, 3, 3, 4, 1, 2, 3, 4],
        };
        let result = TestFixedOpaqueNoPadding::read_xdr(&to_des).unwrap();
        assert_eq!((expected, 8), result);
    }

    #[test]
    fn test_fixed_opaque_no_padding_error() {
        let to_des: Vec<u8> = vec![3, 3, 3, 4, 1, 2, 3];
        let result = TestFixedOpaqueNoPadding::read_xdr(&to_des);
        assert_eq!(Err(Error::BadArraySize), result);
    }

    #[derive(XDRIn, Debug, PartialEq)]
    struct TestFixedOpaquePadding {
        #[array(fixed = 5)]
        pub opaque: Vec<u8>,
    }

    #[test]
    fn test_fixed_opaque_padding() {
        let to_des: Vec<u8> = vec![3, 3, 3, 4, 1, 0, 0, 0];
        let expected = TestFixedOpaquePadding {
            opaque: vec![3, 3, 3, 4, 1],
        };
        let result = TestFixedOpaquePadding::read_xdr(&to_des).unwrap();
        assert_eq!((expected, 8), result);
    }

    #[test]
    fn test_fixed_opaque_padding_error() {
        let to_des: Vec<u8> = vec![3, 3, 3, 4, 1, 0, 0];
        let result = TestFixedOpaquePadding::read_xdr(&to_des);
        assert_eq!(Err(Error::BadArraySize), result);
    }

    #[derive(XDRIn, Debug, PartialEq)]
    struct TestFixedArray {
        #[array(fixed = 3)]
        pub data: Vec<u32>,
    }

    #[test]
    fn test_fixed_array() {
        let to_des: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 3];
        let result = TestFixedArray::read_xdr(&to_des).unwrap();
        let expected = TestFixedArray {
            data: vec![0, 1, 3],
        };
        assert_eq!((expected, 12), result);
    }

    #[test]
    fn test_fixed_array_error() {
        let to_des: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0];
        let result = TestFixedArray::read_xdr(&to_des);
        assert_eq!(Err(Error::UnsignedIntegerBadFormat), result);
    }

    #[test]
    fn test_void() {
        let to_des: Vec<u8> = vec![];
        assert_eq!(((), 0), <()>::read_xdr(&to_des).unwrap());
    }

    #[derive(XDRIn, Debug, PartialEq)]
    struct TestVarArray {
        #[array(var = 3)]
        pub data: Vec<u32>,
    }

    #[test]
    fn test_var_array_limit() {
        let to_des: Vec<u8> = vec![0, 0, 0, 2, 0, 0, 0, 4, 0, 0, 0, 6];
        let result = TestVarArray::read_xdr(&to_des).unwrap();
        let expected = TestVarArray { data: vec![4, 6] };
        assert_eq!((expected, 12), result);
    }

    #[test]
    fn test_var_too_long() {
        let to_des: Vec<u8> = vec![0, 0, 0, 4];
        let result = TestVarArray::read_xdr(&to_des);
        assert_eq!(Err(Error::BadArraySize), result);
    }

    #[derive(XDRIn, Debug, PartialEq)]
    enum TestUnion {
        First(u32),
        Second(TestStruct),
        Third(()),
    }

    #[test]
    fn test_union() {
        let to_des_first: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 3];
        let expected_first = TestUnion::First(3);
        let actual_first = TestUnion::read_xdr(&to_des_first).unwrap();
        assert_eq!((expected_first, 8), actual_first);

        let to_des_second: Vec<u8> = vec![0, 0, 0, 1, 0x3f, 0x80, 0, 0, 0, 0, 0, 2];
        let expected_second = TestUnion::Second(TestStruct { one: 1.0, two: 2 });
        let actual_second = TestUnion::read_xdr(&to_des_second).unwrap();
        assert_eq!((expected_second, 12), actual_second);
    }

    #[test]
    fn test_union_error() {
        let to_des_1: Vec<u8> = vec![0, 0, 0, 3, 0x3f, 0x80, 0, 0, 0, 0, 0, 2];
        assert_eq!(Err(Error::InvalidEnumValue), TestUnion::read_xdr(&to_des_1));

        let to_des_2: Vec<u8> = vec![0, 0, 0, 0, 0x3f, 0x80];
        assert_eq!(
            Err(Error::UnsignedIntegerBadFormat),
            TestUnion::read_xdr(&to_des_2)
        );
    }
}
