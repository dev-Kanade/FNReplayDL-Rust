import requests
import os
import time
import logging
from constants import CONSTANTS

logger = logging.getLogger(__name__)

class ReplayDownloader:
    def __init__(self, auth_handler):
        self.auth = auth_handler
        self.session = requests.Session()

    def get_metadata(self, match_id):
        url = f"{CONSTANTS['meta_data_url']}{match_id}.json"
        resp = self.session.get(url, headers=self.auth.get_headers())
        if resp.status_code != 200:
            return None
        return resp.json()

    def get_download_links(self, match_id, filenames):
        url = f"{CONSTANTS['base_data_url']}{match_id}/"
        resp = self.session.post(
            url,
            json={'files': filenames},
            headers=self.auth.get_headers()
        )
        resp.raise_for_status()
        return resp.json()['files']

    def download_chunk(self, link, info, cache_dir=None, retries=3):
        """チャンクをダウンロードする。失敗した場合は指定回数リトライする"""
        file_id = info.get('Id', 'header')
        cache_path = os.path.join(cache_dir, f"{file_id}.bin") if cache_dir else None

        # キャッシュ（一時ファイル）があればそれを使う
        if cache_path and os.path.exists(cache_path):
            with open(cache_path, 'rb') as f:
                info['data'] = f.read()
            return info

        for attempt in range(retries):
            try:
                resp = self.session.get(link, headers={'User-Agent': 'Tournament replay downloader'}, timeout=10)
                if resp.status_code == 200:
                    info['data'] = resp.content
                    # ダウンロードに成功したらキャッシュに保存
                    if cache_path:
                        with open(cache_path, 'wb') as f:
                            f.write(resp.content)
                    return info
            except requests.exceptions.RequestException:
                pass
            
            if attempt < retries - 1:
                # 失敗時は少し待ってからリトライ
                time.sleep(1 * (attempt + 1))
        
        return None