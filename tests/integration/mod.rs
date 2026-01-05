// Feature flag for tests requiring external services
#[cfg(test)]
pub fn requires_chroma() -> bool {
    std::env::var("CHROMA_URL").is_ok()
}

#[cfg(test)]
pub fn requires_ollama() -> bool {
    std::env::var("OLLAMA_URL").is_ok()
}
