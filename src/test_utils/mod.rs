pub use mock_device::MockDevice;

mod mock_device;

#[macro_export]
macro_rules! test_serde_symmetry {
    ($ty:ty, $input:expr) => {
        #[test]
        fn serde_symmetry() {
            let mut block = $crate::filesystem::Block::new();
            let expected = $input;
            assert_eq!(Ok(<$ty>::SERDE_LEN), expected.serialize(&mut block.writer()));
            let actual = <$ty>::deserialize(&mut block.reader()).unwrap();
            assert_eq!(expected, actual);
        }
    };
}
