import os
import re
import sys
import requests
from pathlib import Path
from natsort import natsorted
from typing import Optional, List, Dict, Any, Tuple
from dataclasses import dataclass

# --- 定数 ---
DEFAULT_TOKEN = 'CiMm8WUfyI0OuH2USrMd9FBz3SGHVezpmj_dt43dGt8'
BASE_URL = 'https://api.annict.com/v1'

@dataclass
class AnnictWork:
    id: int
    title: str
    media: str
    season: str

@dataclass
class AnnictEpisode:
    id: int
    number: Optional[float]
    sort_number: Optional[int]
    number_text: str
    title: str

class AnnictAPI:
    """Annict APIとの通信を担当するクラス"""
    def __init__(self, token: str):
        self.headers = {'Authorization': f'Bearer {token}'}

    def _get(self, endpoint: str, params: Dict[str, Any]) -> Dict[str, Any]:
        try:
            response = requests.get(f"{BASE_URL}/{endpoint}", params=params, headers=self.headers, timeout=10)
            response.raise_for_status()
            return response.json()
        except requests.exceptions.RequestException as e:
            print(f"Error: APIリクエストに失敗しました ({e})")
            return {}

    def search_works(self, title: str) -> List[AnnictWork]:
        params = {'filter_title': title, 'sort_id': 'desc'}
        data = self._get("works", params)
        return [
            AnnictWork(
                id=w['id'],
                title=w['title'],
                media=w.get('media_text', '不明'),
                season=w.get('season_name_text', '不明')
            ) for w in data.get('works', [])
        ]

    def get_episodes(self, work_id: int) -> List[AnnictEpisode]:
        episodes = []
        page = 1
        while True:
            params = {'filter_work_id': work_id, 'sort_id': 'asc', 'page': page, 'per_page': 50}
            data = self._get("episodes", params)
            if not data: break

            for ep in data.get('episodes', []):
                episodes.append(AnnictEpisode(
                    id=ep['id'],
                    number=ep.get('number'),
                    sort_number=ep.get('sort_number'),
                    number_text=ep.get('number_text', ''),
                    title=ep.get('title', '')
                ))

            if not data.get('episodes') or len(episodes) >= data.get('total_count', 0):
                break
            page += 1
        return episodes

class EpisodeMatcher:
    """ファイル名から話数を抽出し、Annictデータと照合するクラス"""
    def __init__(self, episodes: List[AnnictEpisode]):
        self.ep_dict = self._build_index(episodes)

    def _build_index(self, episodes: List[AnnictEpisode]) -> Dict[str, AnnictEpisode]:
        index = {}
        for ep in episodes:
            # number (int/float) を正規化して登録
            if ep.number is not None:
                try:
                    index[str(int(float(ep.number)))] = ep
                except (ValueError, TypeError): pass

            # sort_number (10, 20...) から補完
            if ep.sort_number is not None:
                try:
                    key = str(int(float(ep.sort_number)) // 10)
                    if key not in index:
                        index[key] = ep
                except (ValueError, TypeError): pass
        return index

    def extract_number(self, filename: str) -> Optional[str]:
        name = Path(filename).stem

        # 1. 前処理 (ノイズ除去)
        name = re.sub(r'\[[0-9a-fA-F]{8}\]', '', name) # CRC
        noise = [r'(?i)(?:\d{3,4}p?|(?:10|8)bit|x26[45]|h26[45]|hevc|av1|bdrip|web-dl|tvrip)', r'(?:20[0-2][0-9]|19[8-9][0-9])']
        for p in noise: name = re.sub(p, ' ', name)

        # 2. パターンマッチング (v2対応)
        # 優先度の高いタグ
        tags = [r'(?i)ep(?:isode)?\.?\s*(\d{1,3})(?:v\d+)?', r'第\s*(\d{1,3})(?:v\d+)?\s*[話话]', r'#\s*(\d{1,3})(?:v\d+)?']
        for p in tags:
            match = re.search(p, name)
            if match and str(int(match.group(1))) in self.ep_dict:
                return str(int(match.group(1)))

        # 区切り文字
        delims = [r'[\s\-\_\(\)\[\]](\d{1,3})(?:v\d+)?[\s\-\_\(\)\[\]]', r'^\s*(\d{1,3})(?:v\d+)?[\s\-\_\(\)\[\]]', r'[\s\-\_\(\)\[\]](\d{1,3})(?:v\d+)?\s*$']
        for p in delims:
            matches = re.findall(p, name)
            for m in reversed(matches):
                if str(int(m)) in self.ep_dict: return str(int(m))

        # 最終手段
        potentials = re.findall(r'(\d{1,3})(?:v\d+)?', name)
        for p in reversed(potentials):
            if str(int(p)) in self.ep_dict: return str(int(p))

        return None

class RenamerApp:
    """CLIアプリケーションのメインフローを管理するクラス"""
    def __init__(self):
        token = os.getenv('ANNICT_TOKEN', DEFAULT_TOKEN)
        self.api = AnnictAPI(token)

    def run(self):
        print("=== Annict アニメファイルリネームツール ===")

        # 1. パス入力
        path_input = input("対象のパスを入力: ").strip().strip('"')
        if not path_input: return
        path = Path(path_input)
        if not path.exists():
            print(f"Error: パスが存在しません: {path}")
            return

        # 2. 作品選択
        work = self._select_work()
        if not work: return

        # 3. エピソード取得 & Matcher準備
        print(f"\n「{work.title}」のエピソード情報を取得中...")
        episodes = self.api.get_episodes(work.id)
        matcher = EpisodeMatcher(episodes)
        print(f"DEBUG: {len(matcher.ep_dict)}件の話数をインデックスしました。")

        # 4. ファイルリストアップ
        files = self._get_target_files(path)
        if not files:
            print("対象ファイルが見つかりませんでした。")
            return

        # 5. リネーム案作成
        rename_tasks = self._create_tasks(files, matcher)
        if not rename_tasks:
            print("\n一致する話数が見つかりませんでした。")
            return

        # 6. 実行確認
        self._execute_tasks(rename_tasks)

    def _select_work(self) -> Optional[AnnictWork]:
        while True:
            query = input("\nアニメタイトルを検索: ").strip()
            if not query: return None

            works = self.api.search_works(query)
            if not works:
                print("見つかりませんでした。")
                continue

            for i, w in enumerate(works):
                print(f"{i:2}: {w.title} ({w.media} / {w.season})")

            idx_in = input("\n番号を選択 (r:再検索, Enter:終了): ").strip().lower()
            if idx_in == 'r': continue
            if not idx_in: return None
            try:
                return works[int(idx_in)]
            except (ValueError, IndexError):
                print("無効な番号です。")

    def _get_target_files(self, path: Path) -> List[Path]:
        exts = {'.mp4', '.mkv', '.avi', '.mov', '.wmv'}
        if path.is_file(): return [path]
        return natsorted([f for f in path.iterdir() if f.is_file() and f.suffix.lower() in exts], key=lambda x: x.name)

    def _create_tasks(self, files: List[Path], matcher: EpisodeMatcher) -> List[Tuple[Path, Path]]:
        tasks = []
        print("\n【リネーム案】")
        for f in files:
            num = matcher.extract_number(f.name)
            if num:
                ep = matcher.ep_dict[num]
                title_part = f" - {ep.title}" if ep.title else ""
                new_name = self._sanitize(f"{num.zfill(2)}{title_part}{f.suffix}")
                print(f" {f.name}\n  -> {new_name}")
                tasks.append((f, f.with_name(new_name)))
            else:
                print(f" スキップ: {f.name}")
        return tasks

    def _sanitize(self, name: str) -> str:
        return re.sub(r'[\\/:*?"<>|]', '_', name)

    def _execute_tasks(self, tasks: List[Tuple[Path, Path]]):
        if input(f"\n{len(tasks)}件実行しますか？ (y/n): ").strip().lower() != 'y':
            print("キャンセルしました。")
            return

        success = 0
        for old, new in tasks:
            try:
                if new.exists() and old != new:
                    print(f"Skip: 既に存在します -> {new.name}")
                    continue
                old.rename(new)
                success += 1
            except Exception as e:
                print(f"Error: {old.name} -> {e}")
        print(f"\n完了: {success}/{len(tasks)} 件成功")

if __name__ == "__main__":
    try:
        RenamerApp().run()
    except KeyboardInterrupt:
        print("\n終了します。")