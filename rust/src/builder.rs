use crate::buffer::ReplayBuffer;
use crate::downloader::ReplayDownloader;
use chrono::DateTime;
use serde_json::Value;
use std::collections::HashMap;

pub async fn build_replay(
    meta: &Value,
    downloader: &ReplayDownloader,
    checkpoint: bool,
    event: bool,
    no_data: bool,
) -> Result<Vec<u8>, String> {
    // 1. ダウンロード対象のリスト作成
    let mut files_to_get = vec!["header.bin".to_string()];

    let data_chunks = if !no_data {
        meta.get("DataChunks")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().take(1000).collect::<Vec<_>>())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let event_chunks = if event {
        meta.get("Events")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().take(1000).collect::<Vec<_>>())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let checkpoint_chunks = if checkpoint {
        meta.get("Checkpoints")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().take(1000).collect::<Vec<_>>())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    for c in &data_chunks {
        if let Some(id) = c.get("Id").and_then(|v| v.as_str()) {
            files_to_get.push(format!("{}.bin", id));
        }
    }
    for c in &event_chunks {
        if let Some(id) = c.get("Id").and_then(|v| v.as_str()) {
            files_to_get.push(format!("{}.bin", id));
        }
    }
    for c in &checkpoint_chunks {
        if let Some(id) = c.get("Id").and_then(|v| v.as_str()) {
            files_to_get.push(format!("{}.bin", id));
        }
    }

    let match_id = meta.get("SessionId")
        .and_then(|v| v.as_str())
        .ok_or("メタデータ内にSessionIdが見つかりません")?;

    // 2. ダウンロードリンクを取得
    let links = downloader
        .get_download_links(match_id, files_to_get)
        .await?;

    // 3. ダウンロード用タスク（Partの枠組み）を定義
    struct TaskInfo {
        chunk_type: i32,
        id: String,
        time1: i32,
        time2: i32,
        size_in_bytes: i32,
        group: String,
        metadata: String,
    }

    let mut tasks = Vec::new();
    // Header Task
    tasks.push(TaskInfo {
        chunk_type: 0,
        id: "header".to_string(),
        time1: 0,
        time2: 0,
        size_in_bytes: 0,
        group: String::new(),
        metadata: String::new(),
    });

    // Data Tasks
    for c in &data_chunks {
        tasks.push(TaskInfo {
            chunk_type: 1,
            id: c.get("Id").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            time1: c.get("Time1").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            time2: c.get("Time2").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            size_in_bytes: c.get("SizeInBytes").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            group: String::new(),
            metadata: String::new(),
        });
    }

    // Checkpoint Tasks
    for c in &checkpoint_chunks {
        tasks.push(TaskInfo {
            chunk_type: 2,
            id: c.get("Id").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            time1: c.get("Time1").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            time2: c.get("Time2").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            size_in_bytes: 0,
            group: c.get("Group").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            metadata: c.get("Metadata").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        });
    }

    // Event Tasks
    for c in &event_chunks {
        tasks.push(TaskInfo {
            chunk_type: 3,
            id: c.get("Id").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            time1: c.get("Time1").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            time2: c.get("Time2").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            size_in_bytes: 0,
            group: c.get("Group").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            metadata: c.get("Metadata").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        });
    }

    // 4. 並列ダウンロードの実行
    let downloaded_data = downloader.download_chunks_parallel(&links).await?;

    // 5. メタデータバイナリの生成
    let meta_bin = build_meta_binary(meta)?;

    // 6. .replay ファイルの構築
    let mut replay_buf = ReplayBuffer::new();
    // 最初に meta バイナリを追加
    replay_buf.write_bytes(&meta_bin);

    // 各タスクを結合
    for task in tasks {
        let file_key = if task.id == "header" {
            "header.bin".to_string()
        } else {
            format!("{}.bin", task.id)
        };

        if let Some(data) = downloaded_data.get(&file_key) {
            replay_buf.write_int32(task.chunk_type, None);
            let size_offset = replay_buf.length();
            replay_buf.write_int32(0, None); // サイズ用プレースホルダ
            let start_offset = replay_buf.length();

            match task.chunk_type {
                0 => {
                    replay_buf.write_bytes(data);
                }
                1 => {
                    replay_buf.write_int32(task.time1, None);
                    replay_buf.write_int32(task.time2, None);
                    replay_buf.write_int32(data.len() as i32, None);
                    replay_buf.write_int32(task.size_in_bytes, None);
                    replay_buf.write_bytes(data);
                }
                2 | 3 => {
                    replay_buf.write_string(&task.id);
                    replay_buf.write_string(&task.group);
                    replay_buf.write_string(&task.metadata);
                    replay_buf.write_int32(task.time1, None);
                    replay_buf.write_int32(task.time2, None);
                    replay_buf.write_int32(data.len() as i32, None);
                    replay_buf.write_bytes(data);
                }
                _ => {}
            }

            // チャンクサイズを書き戻し
            let chunk_size = (replay_buf.length() - start_offset) as i32;
            replay_buf.write_int32(chunk_size, Some(size_offset));
        }
    }

    Ok(replay_buf.get_data())
}

pub fn build_meta_binary(header: &Value) -> Result<Vec<u8>, String> {
    let mut buf = ReplayBuffer::new();
    buf.write_int32(0x1CA2E27F, None); // Magic
    buf.write_int32(6, None);          // Version

    let length_in_ms = header.get("LengthInMS").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    buf.write_int32(length_in_ms, None);

    let network_version = header.get("NetworkVersion").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    buf.write_int32(network_version, None);

    buf.write_int32(2147483647, None); // Changelist

    let friendly_name = header.get("FriendlyName").and_then(|v| v.as_str()).unwrap_or("");
    buf.write_string(friendly_name);

    let is_live = header.get("bIsLive").and_then(|v| v.as_bool()).unwrap_or(false);
    buf.write_int32(if is_live { 1 } else { 0 }, None);

    // Timestamp (Ticks 変換)
    let ts = header.get("Timestamp").and_then(|v| v.as_str()).unwrap_or("");
    let ticks = if !ts.is_empty() {
        let dt = DateTime::parse_from_rfc3339(ts)
            .map_err(|e| format!("タイムスタンプのパースに失敗しました {}: {}", ts, e))?;
        (dt.timestamp_millis() * 10_000) + 621_355_968_000_000_000
    } else {
        0
    };
    buf.write_int64(ticks);

    let compressed = header.get("bCompressed").and_then(|v| v.as_bool()).unwrap_or(false);
    buf.write_int32(if compressed { 1 } else { 0 }, None);
    buf.write_int32(0, None);

    // 空の配列の書き込み (lambda b, v: b.write_byte(v))
    buf.write_array::<u8, _>(&[], |b, &v| b.write_byte(v));

    Ok(buf.get_data())
}
