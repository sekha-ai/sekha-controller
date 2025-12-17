pub struct ImportWatcher {
    watch_path: PathBuf,
    processor: Arc<ImportProcessor>,
}

impl ImportWatcher {
    pub async fn watch(&self) -> Result<()> {
        // Use notify crate to watch ~/.sekha/import/
        // Auto-detect ChatGPT/Claude export formats
        // Parse and store in database
        // Move processed files to ~/.sekha/imported/
    }
}