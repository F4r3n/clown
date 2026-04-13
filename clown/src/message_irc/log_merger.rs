//This file is temporary to merge the hashes after a change

use crate::message_irc::message_logger::{LogReader, MessageLogger};
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug)]
struct GroupLogs {
    log_info: (String, String, String),
    files: Vec<PathBuf>,
}

pub fn merge_logs(log_location: PathBuf) -> anyhow::Result<()> {
    //Select all files with the same serverid_channel but different hash
    let list_files = std::fs::read_dir(&log_location)?;
    let mut map: HashMap<String, GroupLogs> = std::collections::HashMap::new();
    for file in list_files.flatten() {
        //parse path name
        if let Some(log_info) = MessageLogger::get_log_info(file.path().as_path()) {
            let key = format!("{}{}", log_info.0, log_info.1);
            map.entry(key)
                .and_modify(|v| v.files.push(file.path()))
                .or_insert(GroupLogs {
                    log_info: (
                        log_info.0.to_string(),
                        log_info.1.to_string(),
                        log_info.2.to_string(),
                    ),
                    files: vec![file.path()],
                });
        }
    }

    //Take the last log and reorder files per timestamp
    for (_, group_log) in map {
        let mut data: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
        let target_filename = MessageLogger::compute_filename(
            group_log.log_info.0.as_str(),
            Some(group_log.log_info.1.as_str()),
        );
        for log in group_log.files {
            let mut reader = LogReader::try_from_path(log.as_path())?;
            if let Ok(last_lines) = reader.read(1)
                && let Some(last_line) = last_lines.first()
            {
                data.push((last_line.time, log.clone()));
            }
        }

        data.sort_by(|a, b| a.0.cmp(&b.0));

        // merge everyting to the last file
        //first create a copy of the current

        let original = log_location.join(target_filename.clone());
        let backup = log_location.join(format!("{}.back", target_filename));
        let temp = log_location.join(format!("{}.temp", target_filename));

        std::fs::copy(&original, backup)?;

        let mut temp_file = std::fs::File::options()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temp)?;

        for file in &data {
            let mut input = std::fs::File::open(&file.1)?;
            std::io::copy(&mut input, &mut temp_file)?;
        }

        for file in data {
            std::fs::remove_file(&file.1)?;
        }
        std::fs::rename(&temp, &original)?;
    }

    Ok(())
}

#[cfg(test)]
mod merge_tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_merge_multiple_files_chronologically() {
        let dir = tempdir().unwrap();
        let log_dir = dir.path().to_path_buf();

        // 1. Create two files that share the same Server and Channel, but different hashes
        // Filename format: server.channel.hash.log
        let server = "libera";
        let chan = "rust";

        let file1_name = MessageLogger::compute_filename(server, Some(chan));
        // We manually create a "clashing" file by changing the hash part slightly
        // or just using different hashes if your compute_filename allows it.
        let file1_path = log_dir.join(&file1_name);
        let file2_path = log_dir.join(format!("{}.{}.12345.log", server, chan));

        // 2. Write data with timestamps
        // File 1: Earlier data
        fs::write(&file1_path, "2026-01-01 10:00:00\tuser1 hello\n").unwrap();
        // File 2: Later data
        fs::write(&file2_path, "2026-01-01 11:00:00\tuser2 world\n").unwrap();

        // 3. Run the merge
        merge_logs(log_dir.clone()).unwrap();
        // 4. Assertions
        let final_log_path = log_dir.join(file1_name);
        assert!(final_log_path.exists(), "Target merged file should exist");

        let content = fs::read_to_string(final_log_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("hello"), "Older entry should be first");
        assert!(lines[1].contains("world"), "Newer entry should be second");

        // Ensure the "partial" file was cleaned up
        assert!(
            !file2_path.exists(),
            "Secondary log file should have been deleted"
        );
    }

    #[test]
    fn test_merge_with_empty_file() {
        let dir = tempdir().unwrap();
        let log_dir = dir.path().to_path_buf();

        let f1 = log_dir.join("srv.chan.hash1.log");
        let f2 = log_dir.join("srv.chan.hash2.log");

        fs::write(&f1, "2026-04-11 12:00:00\tValid log\n").unwrap();
        fs::File::create(&f2).unwrap(); // Empty file

        // merge_logs should handle the error from LogReader gracefully
        // or skip the empty file.
        let _ = merge_logs(log_dir);

        assert!(f1.exists());
    }
}
