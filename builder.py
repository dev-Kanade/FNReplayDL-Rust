from datetime import datetime
from buffer import ReplayBuffer

def build_meta_binary(header):
    """リプレイのメタデータセクションを構築"""
    buf = ReplayBuffer()
    buf.write_int32(0x1CA2E27F) # Magic
    buf.write_int32(6)          # Version
    buf.write_int32(header.get('LengthInMS', 0))
    buf.write_int32(header.get('NetworkVersion', 0))
    buf.write_int32(2147483647) # Changelist
    buf.write_string(header.get('FriendlyName', ''))
    buf.write_int32(1 if header.get('bIsLive') else 0)
    
    # Timestamp (Ticks 変換)
    ts = header.get('Timestamp', '')
    dt = datetime.fromisoformat(ts.replace('Z', '+00:00'))
    ticks = int(dt.timestamp() * 10000000) + 621355968000000000
    buf.write_int64(ticks)
    
    buf.write_int32(1 if header.get('bCompressed') else 0)
    buf.write_int32(0)
    buf.write_array([], lambda b, v: b.write_byte(v))
    return buf.get_data()

def build_replay_file(parts):
    """各チャンクを結合して最終的な .replay ファイルを生成"""
    buf = ReplayBuffer()
    for part in parts:
        if part['type'] == 'meta':
            buf.write_bytes(part['data'])
        elif part['type'] == 'chunk':
            buf.write_int32(part['chunkType'])
            size_offset = buf.length
            buf.write_int32(0) # サイズ用プレースホルダ
            start_offset = buf.length
            
            ct = part['chunkType']
            data = part['data']
            
            if ct == 0: # Header
                buf.write_bytes(data)
            elif ct == 1: # Data
                buf.write_int32(part.get('Time1', 0))
                buf.write_int32(part.get('Time2', 0))
                buf.write_int32(len(data))
                buf.write_int32(part.get('SizeInBytes', 0))
                buf.write_bytes(data)
            elif ct == 2 or ct == 3: # Checkpoint(2) / Event(3)
                buf.write_string(part.get('Id', ''))
                buf.write_string(part.get('Group', ''))
                buf.write_string(part.get('Metadata', ''))
                buf.write_int32(part.get('Time1', 0))
                buf.write_int32(part.get('Time2', 0))
                buf.write_int32(len(data))
                buf.write_bytes(data)
                
            # チャンクサイズを書き戻し
            buf.write_int32(buf.length - start_offset, size_offset)
    return buf.get_data()