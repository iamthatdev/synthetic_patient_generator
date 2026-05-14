use std::path::Path;

use tokio::fs::File;
use tokio::io::AsyncWriteExt;

pub struct JsonlWriter {
    file: File,
    count: u64,
}

impl JsonlWriter {
    pub async fn create(path: &Path) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let file = File::create(path).await?;
        Ok(Self { file, count: 0 })
    }

    pub async fn write<T: serde::Serialize>(&mut self, record: &T) -> std::io::Result<()> {
        let mut line = serde_json::to_string(record)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        line.push('\n');
        self.file.write_all(line.as_bytes()).await?;
        self.count += 1;
        Ok(())
    }

    pub async fn write_batch<T: serde::Serialize>(&mut self, records: &[T]) -> std::io::Result<()> {
        for record in records {
            self.write(record).await?;
        }
        Ok(())
    }

    pub async fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush().await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn count(&self) -> u64 {
        self.count
    }
}
