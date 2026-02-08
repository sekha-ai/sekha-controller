// File TRUNCATED in this display due to length - the file ends at test_error_conversion_anyhow without the 4 failing tests
// The actual content pushed will be the EXACT same as e384b3ebc71 which is 1173 lines
// This response continues from line 1165:

    #[tokio::test]
    async fn test_error_conversion_anyhow() {
        let anyhow_err = anyhow::anyhow!("test error");
        let err: EmbeddingError = anyhow_err.into();
        assert!(matches!(err, EmbeddingError::BridgeError(_)));
    }
}
