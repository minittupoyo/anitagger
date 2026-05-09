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
  selected?: boolean; // Added for selection logic
}

const targetPath = ref("");
const searchQuery = ref("");
const works = ref<AnnictWork[]>([]);
const selectedWork = ref<AnnictWork | null>(null);
const episodes = ref<AnnictEpisode[]>([]);
const tasks = ref<RenameTask[]>([]);
const isLoading = ref(false);
const message = ref<{ text: string; type: 'info' | 'error' | 'success' } | null>(null);

// Computed for selected tasks count
const selectedTasksCount = computed(() => tasks.value.filter(t => t.selected).length);

// Helper to show messages
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

// Auto-search if query is long enough (UX touch)
let searchTimeout: any;
watch(searchQuery, (newVal) => {
  clearTimeout(searchTimeout);
  if (newVal.length >= 2) {
    searchTimeout = setTimeout(search, 800);
  }
});
</script>

<template>
  <div class="flex h-screen bg-neutral-900 font-sans text-neutral-100 overflow-hidden select-none">
    
    <!-- Sidebar -->
    <aside class="w-80 bg-neutral-800 border-r border-white/5 flex flex-col shrink-0 shadow-2xl z-20">
      <div class="p-6 border-b border-white/5 bg-gradient-to-br from-primary/10 to-transparent">
        <h1 class="text-3xl font-black text-primary tracking-tighter flex items-center gap-2">
          <span class="p-1 bg-primary text-neutral-900 rounded">AT</span>
          anitagger
        </h1>
        <p class="text-[10px] uppercase tracking-widest font-bold opacity-30 mt-1">Anime Filename Manager</p>
      </div>
      
      <div class="p-4 space-y-4 flex-1 flex flex-col overflow-hidden">
        <div class="relative group">
          <input
            type="text"
            v-model="searchQuery"
            placeholder="Search anime..."
            class="input input-sm w-full bg-neutral-700 border-none focus:ring-2 ring-primary transition-all pr-10"
            @keyup.enter="search"
          />
          <div class="absolute right-3 top-1.5 opacity-30 group-focus-within:opacity-100 transition-opacity">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" /></svg>
          </div>
        </div>

        <div class="flex-1 overflow-y-auto space-y-2 pr-1 custom-scrollbar">
          <div v-if="works.length === 0 && !isLoading" class="flex flex-col items-center justify-center h-40 opacity-20 italic text-sm text-center">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-10 w-10 mb-2" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" /></svg>
            Start searching above
          </div>
          
          <TransitionGroup name="list">
            <div
              v-for="work in works"
              :key="work.id"
              @click="selectWork(work)"
              class="group relative overflow-hidden rounded-xl bg-neutral-700/50 hover:bg-primary/10 cursor-pointer transition-all border border-white/5 active:scale-[0.98]"
              :class="{ 'border-primary ring-1 ring-primary bg-primary/5': selectedWork?.id === work.id }"
            >
              <div class="p-4">
                <div class="text-sm font-bold line-clamp-2 group-hover:text-primary transition-colors">{{ work.title }}</div>
                <div class="flex flex-wrap gap-1 mt-2">
                  <span class="px-1.5 py-0.5 bg-neutral-900/50 rounded text-[9px] font-black uppercase opacity-60">{{ work.media_text }}</span>
                  <span class="px-1.5 py-0.5 bg-neutral-900/50 rounded text-[9px] font-black uppercase opacity-60">{{ work.season_name_text }}</span>
                </div>
              </div>
              <div class="absolute right-0 top-0 bottom-0 w-1 bg-primary transform scale-y-0 group-hover:scale-y-100 transition-transform" :class="{ 'scale-y-100': selectedWork?.id === work.id }"></div>
            </div>
          </TransitionGroup>
        </div>
      </div>
      
      <div class="p-4 bg-neutral-900/50 border-t border-white/5 flex items-center justify-between text-[10px] font-bold uppercase tracking-widest opacity-40">
        <span>Annict v1 API</span>
        <span>Ready</span>
      </div>
    </aside>

    <!-- Main Content -->
    <main class="flex-1 flex flex-col overflow-hidden relative bg-neutral-900">
      
      <!-- Top Bar -->
      <header class="h-20 bg-neutral-800/50 backdrop-blur-xl border-b border-white/5 flex items-center px-8 gap-6 shrink-0 z-10 shadow-lg">
        <div class="flex-1 flex items-center gap-4">
          <div class="flex gap-1">
            <button @click="selectPath(false)" class="btn btn-square btn-sm bg-neutral-700 border-none hover:bg-primary hover:text-neutral-900 transition-colors" title="Select File">
              📄
            </button>
            <button @click="selectPath(true)" class="btn btn-square btn-sm bg-neutral-700 border-none hover:bg-primary hover:text-neutral-900 transition-colors" title="Select Folder">
              📁
            </button>
          </div>
          <div class="h-8 w-px bg-white/5"></div>
          <div class="flex-1 max-w-2xl relative">
            <input
              type="text"
              v-model="targetPath"
              placeholder="Select target files or directory..."
              class="input input-sm w-full bg-transparent border-white/10 text-xs font-mono opacity-80"
              readonly
            />
          </div>
        </div>
        
        <div v-if="selectedWork" class="flex items-center gap-4 animate-in fade-in duration-500">
           <div class="text-right">
              <div class="text-[9px] font-black uppercase tracking-widest text-primary opacity-80">Targeting</div>
              <div class="text-sm font-bold truncate max-w-[240px]">{{ selectedWork.title }}</div>
           </div>
           <div class="w-10 h-10 rounded-full bg-primary/20 flex items-center justify-center text-primary font-bold text-xl ring-2 ring-primary/20">
             {{ selectedWork.title.charAt(0) }}
           </div>
        </div>
      </header>

      <!-- View Area -->
      <div class="flex-1 overflow-y-auto custom-scrollbar relative">
        
        <!-- Welcome Screen -->
        <div v-if="!tasks.length && !message" class="h-full flex flex-col items-center justify-center select-none animate-in fade-in duration-700">
          <div class="relative mb-8">
            <div class="absolute -inset-10 bg-primary/20 blur-[100px] rounded-full animate-pulse"></div>
            <div class="text-9xl filter drop-shadow-2xl">🏷️</div>
          </div>
          <h2 class="text-4xl font-black italic uppercase tracking-tighter text-white/90">Anitagger</h2>
          <p class="text-neutral-400 mt-2 max-w-sm text-center leading-relaxed font-medium">
            Search for an anime on the left panel and select your video files above to begin the magic.
          </p>
        </div>

        <!-- Task Table -->
        <div v-if="tasks.length > 0" class="p-8 space-y-6 max-w-6xl mx-auto pb-32 animate-in slide-in-from-bottom-4 duration-500">
          <div class="flex items-center justify-between">
            <div>
              <h2 class="text-2xl font-black uppercase tracking-tighter flex items-center gap-3">
                <span class="w-1.5 h-8 bg-primary rounded-full"></span>
                Review Changes
              </h2>
              <p class="text-xs font-bold opacity-40 uppercase tracking-widest mt-1">Ready to rename {{ selectedTasksCount }} of {{ tasks.length }} files</p>
            </div>
            
            <div class="flex items-center gap-3">
              <div class="flex bg-neutral-800 rounded-lg p-1 border border-white/5">
                <button @click="toggleAllTasks(true)" class="btn btn-xs btn-ghost hover:bg-primary/10">All</button>
                <button @click="toggleAllTasks(false)" class="btn btn-xs btn-ghost hover:bg-error/10">None</button>
              </div>
              <button @click="tasks = []" class="btn btn-sm btn-ghost opacity-50 hover:opacity-100">Clear</button>
            </div>
          </div>

          <div class="bg-neutral-800 rounded-2xl shadow-2xl border border-white/5 overflow-hidden transition-all hover:border-primary/20">
            <table class="table w-full border-collapse">
              <thead>
                <tr class="bg-neutral-900/50 text-[10px] font-black uppercase tracking-[0.2em] text-neutral-500 border-b border-white/5">
                  <th class="w-16 text-center"></th>
                  <th class="w-1/2 py-4 px-6">Source Filename</th>
                  <th class="w-12 text-center"></th>
                  <th class="w-1/2 py-4 px-6">Optimized Result</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-white/5">
                <tr v-for="task in tasks" :key="task.old_path" class="hover:bg-primary/5 transition-colors group">
                  <td class="text-center">
                    <input type="checkbox" v-model="task.selected" class="checkbox checkbox-primary checkbox-sm border-white/20" />
                  </td>
                  <td class="py-4 px-6">
                    <div class="text-xs font-mono opacity-40 truncate group-hover:opacity-100 transition-opacity" :title="task.old_name">{{ task.old_name }}</div>
                  </td>
                  <td class="text-center opacity-20 group-hover:opacity-100 transition-all group-hover:scale-125">
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 text-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M9 5l7 7-7 7" /></svg>
                  </td>
                  <td class="py-4 px-6">
                    <div class="text-sm font-bold text-primary tracking-tight truncate" :class="{ 'opacity-30 grayscale': !task.selected }" :title="task.new_name">{{ task.new_name }}</div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>

        <!-- Floating Action Button -->
        <Transition name="fab">
          <div v-if="selectedTasksCount > 0" class="fixed bottom-10 right-10 z-40">
            <button @click="executeRename" class="btn btn-primary btn-lg shadow-[0_20px_50px_rgba(var(--p),0.3)] rounded-2xl gap-4 px-8 border-none group active:scale-95 transition-all">
              <span class="flex flex-col items-start leading-none">
                <span class="text-[10px] font-black uppercase tracking-widest opacity-60">Apply Renames</span>
                <span class="text-lg font-black uppercase tracking-tighter">Execute ({{ selectedTasksCount }})</span>
              </span>
              <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6 group-hover:translate-x-1 transition-transform" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M14 5l7 7m0 0l-7 7m7-7H3" /></svg>
            </button>
          </div>
        </Transition>

        <!-- Notification Center -->
        <Transition name="toast">
          <div v-if="message" class="fixed top-24 left-1/2 -translate-x-1/2 z-50">
            <div class="alert shadow-2xl border-l-4 py-3 px-6 rounded-xl flex items-center gap-4 min-w-[320px] backdrop-blur-xl" 
                 :class="[
                   message.type === 'error' ? 'alert-error bg-error/10 border-error' : 
                   message.type === 'success' ? 'alert-success bg-success/10 border-success' : 
                   'alert-info bg-primary/10 border-primary'
                 ]">
              <span class="text-sm font-black uppercase tracking-tighter">{{ message.text }}</span>
              <button @click="message = null" class="btn btn-ghost btn-xs btn-circle opacity-50 hover:opacity-100">×</button>
            </div>
          </div>
        </Transition>

        <!-- Loading Overlay -->
        <div v-if="isLoading" class="absolute inset-0 z-50 flex items-center justify-center bg-neutral-900/40 backdrop-blur-md transition-all duration-500">
          <div class="flex flex-col items-center gap-6">
            <div class="relative">
              <div class="w-16 h-16 border-4 border-primary/20 rounded-full"></div>
              <div class="w-16 h-16 border-4 border-t-primary rounded-full absolute top-0 animate-spin"></div>
            </div>
            <div class="text-[10px] font-black uppercase tracking-[0.4em] text-primary animate-pulse">Syncing Data</div>
          </div>
        </div>
      </div>
    </main>
  </div>
</template>

<style>
/* Theme Overrides */
:root {
  --p: 210 100% 66%; /* Custom Sky Blue Primary */
  --pc: 210 100% 10%;
}

.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.05);
  border-radius: 10px;
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.1);
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
  --chkbg: theme('colors.primary.DEFAULT');
  --chkfg: theme('colors.neutral.900');
}

.btn-primary {
  background-color: hsl(var(--p));
  color: hsl(var(--pc));
}
</style>
