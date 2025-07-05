pub use mock_device::MockDevice;

mod mock_device;

#[macro_export]
macro_rules! test_serde_symmetry {
    ($ty:ty, $input:expr) => {
        #[test]
        fn serde_symmetry() {
            let mut buf = [0u8; crate::filesystem::Block::LEN * <$ty>::SERDE_BLOCK_COUNT];
            let expected = $input;
            let mut writer = crate::io::Writer::new(&mut buf);
            assert_eq!(Ok(<$ty>::SERDE_LEN), expected.serialize(&mut writer));

            let mut reader = crate::io::Reader::new(&buf);
            let actual = <$ty>::deserialize(&mut reader).unwrap();
            assert_eq!(expected, actual);
        }
    };
}
