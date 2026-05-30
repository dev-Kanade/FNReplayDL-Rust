import os
import argparse
import logging
import shutil
from concurrent.futures import ThreadPoolExecutor
from tqdm import tqdm
from auth import EpicAuth
from downloader import ReplayDownloader
from builder import build_meta_binary, build_replay_file

# ロガーの設定 (ファイルとコンソールの両方に出力)
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s [%(levelname)s] %(message)s',
    handlers=[
        logging.FileHandler("downloader.log", encoding='utf-8'),
        logging.StreamHandler()
    ]
)
logger = logging.getLogger(__name__)

def main():
    parser = argparse.ArgumentParser(description='Fortnite Replay Downloader')
    parser.add_argument('-m', '--match-id', required=True, help='Match/Session ID')
    parser.add_argument('-o', '--output', help='Output directory')
    parser.add_argument('-c', '--checkpoint', action='store_true', help='Download checkpoints')
    parser.add_argument('-e', '--event', action='store_true', help='Download events')
    parser.add_argument('-nd', '--no-data', action='store_true', help='Skip data chunks')

    args = parser.parse_args()
    save_dir = args.output or f"C:/Users/{os.getlogin()}/Downloads/replay-downloader-ReplayFiles"
    if not os.path.exists(save_dir):
        os.makedirs(save_dir, exist_ok=True)

    # チャンク保存用のキャッシュディレクトリ
    cache_dir = os.path.join("cache", args.match_id)
    os.makedirs(cache_dir, exist_ok=True)

    try:
        auth = EpicAuth()
        dl = ReplayDownloader(auth)
        logger.info(f"メタデータを取得中: {args.match_id}")
        meta = dl.get_metadata(args.match_id)
        if not meta:
            logger.error(f"リプレイファイルが存在しません。セッションID ({args.match_id}) を確認してください。")
            return

        # ダウンロード対象のリスト作成
        files_to_get = ['header.bin']
        data_chunks = meta.get('DataChunks', [])[:1000] if not args.no_data else []
        event_chunks = meta.get('Events', [])[:1000] if args.event else []
        checkpoint_chunks = meta.get('Checkpoints', [])[:1000] if args.checkpoint else []
        files_to_get.extend([f"{c['Id']}.bin" for c in data_chunks + event_chunks + checkpoint_chunks])
        logger.info("ダウンロードリンクを取得中...")
        links = dl.get_download_links(args.match_id, files_to_get)
        # タスク構築
        tasks = [{'type': 'chunk', 'chunkType': 0}] # Header
        for c in data_chunks: tasks.append({**c, 'type': 'chunk', 'chunkType': 1})
        for c in checkpoint_chunks: tasks.append({**c, 'type': 'chunk', 'chunkType': 2})
        for c in event_chunks: tasks.append({**c, 'type': 'chunk', 'chunkType': 3})
        def worker(info):
            file_id = f"{info.get('Id', 'header')}.bin"
            if file_id not in links: return None
            return dl.download_chunk(links[file_id]['readLink'], info, cache_dir=cache_dir)

        with ThreadPoolExecutor(max_workers=10) as executor:
            results = list(tqdm(
                executor.map(worker, tasks),
                total=len(tasks),
                unit='chunk',
                desc='ダウンロード中',
                dynamic_ncols=True
            ))
        results = [r for r in results if r]

        if len(results) < len(tasks):
            logger.warning(f"一部のチャンク（{len(tasks) - len(results)}個）の取得に失敗しました。")

        logger.info("リプレイファイルを構築中...")
        meta_bin = build_meta_binary(meta)
        replay_data = build_replay_file([{'type': 'meta', 'data': meta_bin}] + results)
        file_path = os.path.join(save_dir, f"TournamentMatch_{args.match_id}.replay")
        with open(file_path, 'wb') as f:
            f.write(replay_data)
        logger.info(f"完了: {file_path}")

        # 正常に構築できた場合、キャッシュフォルダを削除
        if os.path.exists(cache_dir):
            shutil.rmtree(cache_dir)
            logger.info(f"キャッシュを削除しました: {cache_dir}")

    except Exception as e:
        logger.exception(f"実行中に予期せぬエラーが発生しました: {e}")

if __name__ == "__main__":
    main()