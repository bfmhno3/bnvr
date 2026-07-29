use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;

pub struct LogReader {
    file_path: PathBuf,
    file: Option<File>,
    last_position: u64,
}

impl LogReader {
    pub fn new(log_path: PathBuf) -> Self {
        Self {
            file_path: log_path,
            file: None,
            last_position: 0,
        }
    }

    pub fn read_new_lines(&mut self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        self.ensure_file()?;
        let Some(file) = self.file.as_mut() else {
            return Ok(Vec::new());
        };
        let len = file.metadata()?.len();
        if len < self.last_position {
            self.last_position = 0;
        }
        file.seek(SeekFrom::Start(self.last_position))?;
        let mut reader = BufReader::new(file);
        let mut lines = Vec::new();
        let mut line = String::new();
        loop {
            line.clear();
            let bytes = reader.read_line(&mut line)?;
            if bytes == 0 {
                break;
            }
            lines.push(line.trim_end_matches(['\r', '\n']).to_string());
        }
        self.last_position = reader.stream_position()?;
        Ok(lines)
    }

    pub fn read_tail(&mut self, count: usize) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let content = match std::fs::read_to_string(&self.file_path) {
            Ok(content) => content,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        };
        let lines: Vec<String> = content
            .lines()
            .rev()
            .take(count)
            .map(str::to_string)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        self.last_position = u64::try_from(content.len()).unwrap_or_default();
        self.file = File::open(&self.file_path).ok();
        Ok(lines)
    }

    fn ensure_file(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.file.is_none() {
            self.file = match File::open(&self.file_path) {
                Ok(file) => Some(file),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) => return Err(e.into()),
            };
        }
        Ok(())
    }
}
