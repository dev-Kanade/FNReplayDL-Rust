import requests
import os
import json
from constants import CONSTANTS

class EpicAuth:
    def __init__(self):
        self.access_token = None
        self.session = requests.Session()

    def authenticate(self):
        # キャッシュの確認
        if os.path.exists(CONSTANTS['cache_file']):
            with open(CONSTANTS['cache_file'], 'r') as f:
                cache = json.load(f)
            token_str = f"{cache['token_type']} {cache['access_token']}"
            # トークンの有効性を確認
            resp = self.session.post(CONSTANTS['verify_endpoint'], headers={'Authorization': token_str})
            if resp.status_code == 200:
                self.access_token = token_str
                return token_str

        # 新規取得
        data = {
            "grant_type": "client_credentials",
            "token_type": "eg1"
        }
        auth = (CONSTANTS['auth_client_id'], CONSTANTS['auth_client_secret'])
        response = self.session.post(CONSTANTS['token_endpoint'], data=data, auth=auth)
        response.raise_for_status()
        data = response.json()
        
        with open(CONSTANTS['cache_file'], 'w') as f:
            json.dump(data, f)

        self.access_token = f"{data['token_type']} {data['access_token']}"
        return self.access_token

    def get_headers(self):
        if not self.access_token:
            self.authenticate()
        return {"Authorization": self.access_token, "User-Agent": CONSTANTS['user_agent']}