use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use vpt_application::ports::{Prompt, PromptError};

pub struct TtyPrompt {
    tty: File,
}

impl TtyPrompt {
    pub fn open() -> Result<Self, PromptError> {
        let tty = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")
            .map_err(|_| PromptError::NoTerminal)?;
        Ok(Self { tty })
    }
}

impl Prompt for TtyPrompt {
    fn choose(
        &mut self,
        question: &str,
        options: &[&str],
        preselected: usize,
    ) -> Result<usize, PromptError> {
        let mut text = format!("{question}:\n");
        for (index, option) in options.iter().enumerate() {
            let marker = if index == preselected { "*" } else { " " };
            text.push_str(&format!("  {marker} {} {option}\n", index + 1));
        }
        text.push_str(&format!("choice [{}]: ", preselected + 1));
        self.tty
            .write_all(text.as_bytes())
            .map_err(|_| PromptError::Closed)?;
        let mut answer = String::new();
        let mut reader = BufReader::new(self.tty.try_clone().map_err(|_| PromptError::Closed)?);
        if reader
            .read_line(&mut answer)
            .map_err(|_| PromptError::Closed)?
            == 0
        {
            return Err(PromptError::Closed);
        }
        let answer = answer.trim();
        if answer.is_empty() {
            return Ok(preselected);
        }
        match answer.parse::<usize>() {
            Ok(number) if (1..=options.len()).contains(&number) => Ok(number - 1),
            _ => Ok(preselected),
        }
    }
}
