<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

interface AnnictWork {
  id: number;
  title: string;
  media_text: string | null;
  season_name_text: string | null;
}

interface AnnictEpisode {
  id: number;
  number: number | null;
  sort_number: number | null;
  number_text: string | null;
  title: string | null;
}

interface RenameTask {
  old_path: string;
  new_path: string;
  old_name: string;
  new_name: string;
}

const targetPath = ref("");
const searchQuery = ref("");
const works = ref<AnnictWork[]>([]);
const selectedWork = ref<AnnictWork | null>(null);
const episodes = ref<AnnictEpisode[]>([]);
const tasks = ref<RenameTask[]>([]);
const isLoading = ref(false);
const message = ref("");

async function selectPath(isDirectory: boolean) {
  const selected = await open({
    directory: isDirectory,
    multiple: false,
    filters: isDirectory ? [] : [{ name: "Video", extensions: ["mp4", "mkv", "avi", "mov", "wmv"] }]
  });
  if (selected && typeof selected === "string") {
    targetPath.value = selected;
    if (selectedWork.value) {
      await generateTasks();
    }
  }
}

async function search() {
  if (!searchQuery.value) return;
  isLoading.value = true;
  message.value = "";
  try {
    works.value = await invoke("search_works", { query: searchQuery.value });
    if (works.value.length === 0) {
      message.value = "作品が見つかりませんでした。";
    }
  } catch (e) {
    message.value = `エラー: ${e}`;
  } finally {
    isLoading.value = false;
  }
}

async function selectWork(work: AnnictWork) {
  selectedWork.value = work;
  isLoading.value = true;
  message.value = "";
  try {
    episodes.value = await invoke("get_episodes", { workId: work.id });
    await generateTasks();
  } catch (e) {
    message.value = `エラー: ${e}`;
  } finally {
    isLoading.value = false;
  }
}

async function generateTasks() {
  if (!targetPath.value || !episodes.value.length) return;
  try {
    tasks.value = await invoke("get_rename_tasks", {
      path: targetPath.value,
      episodes: episodes.value,
    });
    if (tasks.value.length === 0) {
      message.value = "一致する話数が見つかりませんでした。";
    }
  } catch (e) {
    message.value = `エラー: ${e}`;
  }
}

async function executeRename() {
  if (tasks.value.length === 0) return;
  isLoading.value = true;
  try {
    const successCount: number = await invoke("execute_renames", {
      tasks: tasks.value,
    });
    message.value = `${successCount}件のリネームが完了しました。`;
    tasks.value = [];
    selectedWork.value = null;
    episodes.value = [];
    targetPath.value = "";
  } catch (e) {
    message.value = `エラー: ${e}`;
  } finally {
    isLoading.value = false;
  }
}
</script>

<template>
  <div class="flex h-screen bg-base-200 font-sans text-base-content overflow-hidden">
    <!-- Sidebar (Search & Work Selection) -->
    <aside class="w-80 bg-base-100 border-r border-base-300 flex flex-col shrink-0 shadow-lg">
      <div class="p-4 border-b border-base-300 bg-primary/5">
        <h1 class="text-2xl font-black text-primary tracking-tight">anitagger</h1>
      </div>
      
      <div class="p-4 space-y-4 flex-1 flex flex-col overflow-hidden">
        <div class="form-control">
          <label class="label"><span class="label-text font-bold">1. アニメを検索</span></label>
          <div class="join w-full">
            <input
              type="text"
              v-model="searchQuery"
              placeholder="タイトルを入力..."
              class="input input-bordered join-item w-full input-sm"
              @keyup.enter="search"
            />
            <button @click="search" class="btn btn-primary join-item btn-sm" :disabled="isLoading">
              <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" /></svg>
            </button>
          </div>
        </div>

        <div class="flex-1 overflow-y-auto space-y-2 pr-1 custom-scrollbar">
          <div v-if="works.length === 0 && !isLoading" class="text-center py-10 opacity-30 italic text-sm">
            検索結果がここに表示されます
          </div>
          <div
            v-for="work in works"
            :key="work.id"
            @click="selectWork(work)"
            class="card bg-base-200 hover:bg-primary/10 cursor-pointer transition-all border border-transparent"
            :class="{ 'border-primary ring-1 ring-primary bg-primary/5': selectedWork?.id === work.id }"
          >
            <div class="p-3">
              <div class="text-sm font-bold line-clamp-2">{{ work.title }}</div>
              <div class="flex gap-2 mt-1 opacity-60 text-[10px] uppercase font-bold tracking-widest">
                <span>{{ work.media_text }}</span>
                <span>•</span>
                <span>{{ work.season_name_text }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </aside>

    <!-- Main Content -->
    <main class="flex-1 flex flex-col overflow-hidden relative">
      <!-- Top Header -->
      <header class="bg-base-100 h-16 border-b border-base-300 flex items-center px-6 gap-4 shrink-0 shadow-sm">
        <div class="flex-1 flex items-center gap-2">
          <div class="badge badge-outline gap-2 p-3 font-medium opacity-70">
            2. 対象を選択
          </div>
          <div class="join flex-1 max-w-xl">
            <input
              type="text"
              v-model="targetPath"
              placeholder="ファイルまたはフォルダを選択してください"
              class="input input-bordered join-item w-full input-sm"
              readonly
            />
            <button @click="selectPath(false)" class="btn btn-outline join-item btn-sm" title="ファイルを選択">
               📄
            </button>
            <button @click="selectPath(true)" class="btn btn-outline join-item btn-sm" title="フォルダを選択">
               📁
            </button>
          </div>
        </div>
        
        <div class="flex items-center gap-2">
           <div v-if="selectedWork" class="text-xs text-right max-w-[200px]">
              <div class="opacity-50">選択中:</div>
              <div class="font-bold truncate">{{ selectedWork.title }}</div>
           </div>
        </div>
      </header>

      <!-- Content Area -->
      <div class="flex-1 overflow-y-auto p-6 custom-scrollbar">
        <div v-if="!tasks.length && !message" class="h-full flex flex-col items-center justify-center opacity-20 select-none">
          <div class="text-8xl mb-4">🏷️</div>
          <div class="text-xl font-black italic uppercase tracking-tighter">Ready to tag your anime</div>
          <p class="text-sm">左側のパネルからアニメを検索し、上のバーからファイルを選択してください</p>
        </div>

        <div v-if="tasks.length > 0" class="space-y-4 animate-in fade-in slide-in-from-bottom-2 duration-300">
          <div class="flex items-center justify-between">
            <h2 class="text-lg font-black uppercase tracking-tight flex items-center gap-2">
              <span class="w-2 h-6 bg-success"></span>
              リネーム候補 ({{ tasks.length }}件)
            </h2>
            <div class="flex gap-2">
              <button @click="tasks = []" class="btn btn-ghost btn-sm">クリア</button>
              <button @click="executeRename" class="btn btn-success btn-sm px-8" :disabled="isLoading">
                実行する
              </button>
            </div>
          </div>

          <div class="bg-base-100 rounded-xl shadow-sm border border-base-300 overflow-hidden">
            <table class="table table-zebra w-full">
              <thead class="bg-base-200">
                <tr>
                  <th class="w-1/2">元のファイル名</th>
                  <th class="w-12 text-center opacity-30">➜</th>
                  <th class="w-1/2">新しいファイル名</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="task in tasks" :key="task.old_path" class="hover">
                  <td>
                    <div class="text-xs font-mono opacity-50 truncate" :title="task.old_name">{{ task.old_name }}</div>
                  </td>
                  <td class="text-center opacity-30">➜</td>
                  <td>
                    <div class="text-sm font-bold text-success truncate" :title="task.new_name">{{ task.new_name }}</div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>

        <!-- Status Alert -->
        <div v-if="message" class="alert shadow-sm mt-4 border-l-4" :class="message.startsWith('エラー') ? 'alert-error' : 'alert-info'">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" class="stroke-current shrink-0 w-5 h-5"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>
          <span class="text-sm font-medium">{{ message }}</span>
          <button @click="message = ''" class="btn btn-ghost btn-xs">×</button>
        </div>
      </div>

      <!-- Loading Overlay -->
      <div v-if="isLoading" class="absolute inset-0 z-50 flex items-center justify-center bg-base-100/60 backdrop-blur-[2px] transition-all duration-300">
        <div class="flex flex-col items-center gap-4 bg-base-100 p-8 rounded-2xl shadow-2xl border border-base-300">
          <span class="loading loading-spinner loading-lg text-primary"></span>
          <div class="text-xs font-bold uppercase tracking-widest opacity-50">Processing...</div>
        </div>
      </div>
    </main>
  </div>
</template>

<style>
.custom-scrollbar::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: hsl(var(--bc) / 0.1);
  border-radius: 10px;
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: hsl(var(--bc) / 0.2);
}

/* Transitions */
.animate-in {
  animation-duration: 0.3s;
  animation-fill-mode: both;
}
@keyframes fade-in {
  from { opacity: 0; }
  to { opacity: 1; }
}
@keyframes slide-in-from-bottom-2 {
  from { transform: translateY(0.5rem); }
  to { transform: translateY(0); }
}
.fade-in { animation-name: fade-in; }
.slide-in-from-bottom-2 { animation-name: slide-in-from-bottom-2; }

/* Line Clamp */
.line-clamp-2 {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
</style>
