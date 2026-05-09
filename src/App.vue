<script setup lang="ts">
import { ref, computed, watch } from "vue";
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
  selected?: boolean;
}

const targetPath = ref("");
const searchQuery = ref("");
const works = ref<AnnictWork[]>([]);
const selectedWork = ref<AnnictWork | null>(null);
const episodes = ref<AnnictEpisode[]>([]);
const tasks = ref<RenameTask[]>([]);
const isLoading = ref(false);
const message = ref<{ text: string; type: 'info' | 'error' | 'success' } | null>(null);

const selectedTasksCount = computed(() => tasks.value.filter(t => t.selected).length);

const showMsg = (text: string, type: 'info' | 'error' | 'success' = 'info') => {
  message.value = { text, type };
  setTimeout(() => {
    if (message.value?.text === text) message.value = null;
  }, 5000);
};

async function selectPath(isDirectory: boolean) {
  const selected = await open({
    directory: isDirectory,
    multiple: false,
    filters: isDirectory ? [] : [{ name: "ビデオファイル", extensions: ["mp4", "mkv", "avi", "mov", "wmv"] }]
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
  message.value = null;
  try {
    works.value = await invoke("search_works", { query: searchQuery.value });
    if (works.value.length === 0) {
      showMsg("作品が見つかりませんでした。", "info");
    }
  } catch (e) {
    showMsg(`検索エラー: ${e}`, "error");
  } finally {
    isLoading.value = false;
  }
}

async function selectWork(work: AnnictWork) {
  if (selectedWork.value?.id === work.id) return;
  selectedWork.value = work;
  isLoading.value = true;
  message.value = null;
  try {
    episodes.value = await invoke("get_episodes", { workId: work.id });
    await generateTasks();
  } catch (e) {
    showMsg(`情報取得エラー: ${e}`, "error");
  } finally {
    isLoading.value = false;
  }
}

async function generateTasks() {
  if (!targetPath.value || !episodes.value.length) return;
  try {
    const rawTasks: RenameTask[] = await invoke("get_rename_tasks", {
      path: targetPath.value,
      episodes: episodes.value,
    });
    tasks.value = rawTasks.map(t => ({ ...t, selected: true }));
    if (tasks.value.length === 0) {
      showMsg("一致する話数が見つかりませんでした。", "info");
    }
  } catch (e) {
    showMsg(`タスク生成エラー: ${e}`, "error");
  }
}

async function executeRename() {
  const tasksToExecute = tasks.value.filter(t => t.selected);
  if (tasksToExecute.length === 0) return;
  
  isLoading.value = true;
  try {
    const successCount: number = await invoke("execute_renames", {
      tasks: tasksToExecute,
    });
    showMsg(`${successCount}件のリネームを完了しました。`, "success");
    tasks.value = [];
    selectedWork.value = null;
    episodes.value = [];
    targetPath.value = "";
    searchQuery.value = "";
    works.value = [];
  } catch (e) {
    showMsg(`実行エラー: ${e}`, "error");
  } finally {
    isLoading.value = false;
  }
}

const toggleAllTasks = (val: boolean) => {
  tasks.value.forEach(t => t.selected = val);
};

let searchTimeout: any;
watch(searchQuery, (newVal) => {
  clearTimeout(searchTimeout);
  if (newVal.length >= 2) {
    searchTimeout = setTimeout(search, 800);
  }
});
</script>

<template>
  <div class="flex h-screen bg-slate-50 font-sans text-slate-900 overflow-hidden select-none">
    
    <!-- サイドバー -->
    <aside class="w-80 bg-white border-r border-slate-200 flex flex-col shrink-0 shadow-sm z-20">
      <div class="p-6 border-b border-slate-100 bg-slate-50/50">
        <h1 class="text-2xl font-black text-blue-600 tracking-tighter flex items-center gap-2">
          <span class="p-1 bg-blue-600 text-white rounded shadow-sm">AT</span>
          anitagger
        </h1>
        <p class="text-[10px] uppercase tracking-widest font-bold opacity-40 mt-1">アニメファイル管理ツール</p>
      </div>
      
      <div class="p-4 space-y-4 flex-1 flex flex-col overflow-hidden">
        <div class="relative group">
          <input
            type="text"
            v-model="searchQuery"
            placeholder="アニメを検索..."
            class="input input-sm w-full bg-slate-100 border-none focus:ring-2 ring-blue-500/50 transition-all pr-10"
            @keyup.enter="search"
          />
          <div class="absolute right-3 top-1.5 opacity-30 group-focus-within:opacity-100 transition-opacity">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" /></svg>
          </div>
        </div>

        <div class="flex-1 overflow-y-auto space-y-2 pr-1 custom-scrollbar">
          <div v-if="works.length === 0 && !isLoading" class="flex flex-col items-center justify-center h-40 opacity-30 italic text-sm text-center">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-10 w-10 mb-2" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" /></svg>
            上の検索窓から開始
          </div>
          
          <TransitionGroup name="list">
            <div
              v-for="work in works"
              :key="work.id"
              @click="selectWork(work)"
              class="group relative overflow-hidden rounded-xl bg-slate-50 hover:bg-blue-50 cursor-pointer transition-all border border-slate-200 active:scale-[0.98]"
              :class="{ 'border-blue-500 ring-1 ring-blue-500 bg-blue-50/50': selectedWork?.id === work.id }"
            >
              <div class="p-4">
                <div class="text-sm font-bold line-clamp-2 group-hover:text-blue-600 transition-colors">{{ work.title }}</div>
                <div class="flex flex-wrap gap-1 mt-2">
                  <span class="px-1.5 py-0.5 bg-white border border-slate-200 rounded text-[9px] font-black uppercase text-slate-500">{{ work.media_text }}</span>
                  <span class="px-1.5 py-0.5 bg-white border border-slate-200 rounded text-[9px] font-black uppercase text-slate-500">{{ work.season_name_text }}</span>
                </div>
              </div>
              <div class="absolute left-0 top-0 bottom-0 w-1 bg-blue-500 transform scale-y-0 group-hover:scale-y-100 transition-transform" :class="{ 'scale-y-100': selectedWork?.id === work.id }"></div>
            </div>
          </TransitionGroup>
        </div>
      </div>
      
      <div class="p-4 bg-slate-50 border-t border-slate-200 flex items-center justify-between text-[10px] font-bold uppercase tracking-widest opacity-50">
        <span>Annict v1 API</span>
        <span>準備完了</span>
      </div>
    </aside>

    <!-- メインコンテンツ -->
    <main class="flex-1 flex flex-col overflow-hidden relative">
      
      <!-- トップバー -->
      <header class="h-20 bg-white/80 backdrop-blur-xl border-b border-slate-200 flex items-center px-8 gap-6 shrink-0 z-10 shadow-sm">
        <div class="flex-1 flex items-center gap-4">
          <div class="flex gap-1">
            <button @click="selectPath(false)" class="btn btn-square btn-sm bg-slate-100 border-none hover:bg-blue-600 hover:text-white transition-colors" title="ファイルを選択">
              📄
            </button>
            <button @click="selectPath(true)" class="btn btn-square btn-sm bg-slate-100 border-none hover:bg-blue-600 hover:text-white transition-colors" title="フォルダを選択">
              📁
            </button>
          </div>
          <div class="h-8 w-px bg-slate-200"></div>
          <div class="flex-1 max-w-2xl relative">
            <input
              type="text"
              v-model="targetPath"
              placeholder="ファイルまたはフォルダを選択してください..."
              class="input input-sm w-full bg-slate-50 border-slate-200 text-xs font-mono text-slate-600"
              readonly
            />
          </div>
        </div>
        
        <div v-if="selectedWork" class="flex items-center gap-4 animate-in fade-in duration-500">
           <div class="text-right">
              <div class="text-[9px] font-black uppercase tracking-widest text-blue-600 opacity-80">選択中の作品</div>
              <div class="text-sm font-bold truncate max-w-[240px]">{{ selectedWork.title }}</div>
           </div>
           <div class="w-10 h-10 rounded-full bg-blue-100 flex items-center justify-center text-blue-600 font-bold text-xl ring-2 ring-blue-50">
             {{ selectedWork.title.charAt(0) }}
           </div>
        </div>
      </header>

      <!-- 表示エリア -->
      <div class="flex-1 overflow-y-auto custom-scrollbar relative">
        
        <!-- 初期画面 -->
        <div v-if="!tasks.length && !message" class="h-full flex flex-col items-center justify-center select-none animate-in fade-in duration-700">
          <div class="relative mb-8">
            <div class="absolute -inset-10 bg-blue-400/10 blur-[100px] rounded-full"></div>
            <div class="text-9xl filter drop-shadow-xl">🏷️</div>
          </div>
          <h2 class="text-4xl font-black italic uppercase tracking-tighter text-slate-800">Anitagger</h2>
          <p class="text-slate-500 mt-2 max-w-sm text-center leading-relaxed font-medium">
            左パネルからアニメを検索し、上部バーからファイルを選択して開始してください。
          </p>
        </div>

        <!-- タスクテーブル -->
        <div v-if="tasks.length > 0" class="p-8 space-y-6 max-w-6xl mx-auto pb-32 animate-in slide-in-from-bottom-4 duration-500">
          <div class="flex items-center justify-between">
            <div>
              <h2 class="text-2xl font-black uppercase tracking-tighter flex items-center gap-3 text-slate-800">
                <span class="w-1.5 h-8 bg-blue-600 rounded-full"></span>
                リネーム内容の確認
              </h2>
              <p class="text-xs font-bold opacity-50 uppercase tracking-widest mt-1">{{ tasks.length }}件中 {{ selectedTasksCount }}件を選択中</p>
            </div>
            
            <div class="flex items-center gap-3">
              <div class="flex bg-slate-100 rounded-lg p-1 border border-slate-200">
                <button @click="toggleAllTasks(true)" class="btn btn-xs btn-ghost hover:bg-white">すべて</button>
                <button @click="toggleAllTasks(false)" class="btn btn-xs btn-ghost hover:bg-white">解除</button>
              </div>
              <button @click="tasks = []" class="btn btn-sm btn-ghost opacity-50 hover:opacity-100">クリア</button>
            </div>
          </div>

          <div class="bg-white rounded-2xl shadow-sm border border-slate-200 overflow-hidden transition-all hover:shadow-md">
            <table class="table w-full border-collapse">
              <thead>
                <tr class="bg-slate-50 text-[10px] font-black uppercase tracking-[0.2em] text-slate-400 border-b border-slate-100">
                  <th class="w-16 text-center">選択</th>
                  <th class="w-1/2 py-4 px-6">現在のファイル名</th>
                  <th class="w-12 text-center"></th>
                  <th class="w-1/2 py-4 px-6">リネーム後のファイル名</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-100">
                <tr v-for="task in tasks" :key="task.old_path" class="hover:bg-blue-50/30 transition-colors group">
                  <td class="text-center">
                    <input type="checkbox" v-model="task.selected" class="checkbox checkbox-primary checkbox-sm border-slate-300" />
                  </td>
                  <td class="py-4 px-6">
                    <div class="text-xs font-mono opacity-50 truncate group-hover:opacity-100 transition-opacity" :title="task.old_name">{{ task.old_name }}</div>
                  </td>
                  <td class="text-center opacity-20 group-hover:opacity-100 transition-all group-hover:scale-125">
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 text-blue-600" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M9 5l7 7-7 7" /></svg>
                  </td>
                  <td class="py-4 px-6">
                    <div class="text-sm font-bold text-blue-600 tracking-tight truncate" :class="{ 'opacity-30 grayscale': !task.selected }" :title="task.new_name">{{ task.new_name }}</div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>

        <!-- 実行ボタン（フローティング） -->
        <Transition name="fab">
          <div v-if="selectedTasksCount > 0" class="fixed bottom-10 right-10 z-40">
            <button @click="executeRename" class="btn btn-primary btn-lg shadow-xl shadow-blue-500/20 rounded-2xl gap-4 px-8 border-none group active:scale-95 transition-all bg-blue-600 hover:bg-blue-700 text-white">
              <span class="flex flex-col items-start leading-none">
                <span class="text-[10px] font-black uppercase tracking-widest opacity-70">リネームを実行</span>
                <span class="text-lg font-black uppercase tracking-tighter">実行 ({{ selectedTasksCount }}件)</span>
              </span>
              <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6 group-hover:translate-x-1 transition-transform" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M14 5l7 7m0 0l-7 7m7-7H3" /></svg>
            </button>
          </div>
        </Transition>

        <!-- 通知 -->
        <Transition name="toast">
          <div v-if="message" class="fixed top-24 left-1/2 -translate-x-1/2 z-50">
            <div class="alert shadow-xl border-l-4 py-3 px-6 rounded-xl flex items-center gap-4 min-w-[320px] backdrop-blur-xl bg-white" 
                 :class="[
                   message.type === 'error' ? 'border-red-500 text-red-700 bg-red-50' : 
                   message.type === 'success' ? 'border-green-500 text-green-700 bg-green-50' : 
                   'border-blue-500 text-blue-700 bg-blue-50'
                 ]">
              <span class="text-sm font-black uppercase tracking-tighter">{{ message.text }}</span>
              <button @click="message = null" class="btn btn-ghost btn-xs btn-circle opacity-50 hover:opacity-100">×</button>
            </div>
          </div>
        </Transition>

        <!-- ローディング -->
        <div v-if="isLoading" class="absolute inset-0 z-50 flex items-center justify-center bg-white/60 backdrop-blur-sm transition-all duration-500">
          <div class="flex flex-col items-center gap-6">
            <div class="relative">
              <div class="w-16 h-16 border-4 border-blue-100 rounded-full"></div>
              <div class="w-16 h-16 border-4 border-t-blue-600 rounded-full absolute top-0 animate-spin"></div>
            </div>
            <div class="text-[10px] font-black uppercase tracking-[0.4em] text-blue-600 animate-pulse">データを同期中</div>
          </div>
        </div>
      </div>
    </main>
  </div>
</template>

<style>
.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.05);
  border-radius: 10px;
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: rgba(0, 0, 0, 0.1);
}

/* Transitions */
.list-enter-active, .list-leave-active { transition: all 0.4s ease; }
.list-enter-from, .list-leave-to { opacity: 0; transform: translateX(-20px); }

.fab-enter-active, .fab-leave-active { transition: all 0.5s cubic-bezier(0.175, 0.885, 0.32, 1.275); }
.fab-enter-from, .fab-leave-to { opacity: 0; transform: scale(0.5) translateY(50px); }

.toast-enter-active, .toast-leave-active { transition: all 0.3s ease; }
.toast-enter-from, .toast-leave-to { opacity: 0; transform: translate(-50%, -20px); }

.animate-in {
  animation-duration: 0.6s;
  animation-fill-mode: both;
}
@keyframes fade-in { from { opacity: 0; } to { opacity: 1; } }
@keyframes slide-in-from-bottom-4 { from { transform: translateY(2rem); opacity: 0; } to { transform: translateY(0); opacity: 1; } }
.fade-in { animation-name: fade-in; }
.slide-in-from-bottom-4 { animation-name: slide-in-from-bottom-4; }

/* Truncation */
.line-clamp-2 {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

input[type="checkbox"].checkbox {
  --chkbg: #2563eb;
  --chkfg: #ffffff;
}
</style>
