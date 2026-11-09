// 選択中 bin のソースを AtCoder 提出用にコピーする下部ドック。
// コピー時にソースを取り直すので、選択後にファイルを編集しても最新の内容が入る。

async function fetchSolverSource(binName) {
  const res = await fetch(`/api/solver-source?binName=${encodeURIComponent(binName)}`);
  const payload = await res.json();
  if (!res.ok) {
    throw new Error(payload.error || `HTTP ${res.status}`);
  }
  return payload;
}

function formatBytes(bytes) {
  return bytes >= 1024 ? `${(bytes / 1024).toFixed(1)} KiB` : `${bytes} B`;
}

function formatMtime(iso) {
  const date = new Date(iso);
  const pad = (value) => String(value).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

export function createSourcePanel(els, { onClose }) {
  let binName = "";
  let requestSeq = 0;
  let feedbackTimer = 0;

  function setFeedback(text, kind = "") {
    clearTimeout(feedbackTimer);
    els.sourceFeedback.textContent = text;
    els.sourceFeedback.dataset.kind = kind;
    if (kind === "ok") {
      feedbackTimer = setTimeout(() => {
        els.sourceFeedback.textContent = "";
      }, 2000);
    }
  }

  function renderSource(payload) {
    els.sourcePath.textContent = payload.path;
    els.sourceMeta.textContent = `${payload.lines} lines · ${formatBytes(payload.bytes)} · ${formatMtime(payload.mtime)}`;
    els.sourceWarnings.textContent = "";
    for (const warning of payload.warnings) {
      const item = document.createElement("li");
      item.textContent = warning;
      els.sourceWarnings.appendChild(item);
    }
    els.sourceWarnings.hidden = payload.warnings.length === 0;
    els.sourcePreview.textContent = payload.source;
    els.sourceCopy.disabled = false;
  }

  function renderError(error) {
    els.sourcePath.textContent = "";
    els.sourceMeta.textContent = "";
    els.sourceWarnings.hidden = true;
    els.sourcePreview.textContent = "";
    els.sourceCopy.disabled = true;
    setFeedback(String(error.message ?? error), "error");
  }

  async function load() {
    const seq = ++requestSeq;
    try {
      const payload = await fetchSolverSource(binName);
      if (seq === requestSeq) {
        renderSource(payload);
      }
      return payload;
    } catch (error) {
      if (seq === requestSeq) {
        renderError(error);
      }
      throw error;
    }
  }

  async function copy() {
    if (!binName) {
      return;
    }
    try {
      // fetch を待ってから writeText するとクリック由来の権限が切れる (Safari) ので、
      // 取得中の Promise を ClipboardItem に渡して、クリック中に書き込みを開始する。
      const loading = load();
      await navigator.clipboard.write([
        new ClipboardItem({
          "text/plain": loading.then((payload) => new Blob([payload.source], { type: "text/plain" })),
        }),
      ]);
      const payload = await loading;
      setFeedback(`コピーした (${payload.lines} lines)`, "ok");
    } catch (error) {
      setFeedback(`コピー失敗: ${error.message ?? error}`, "error");
    }
  }

  function setPreviewOpen(open) {
    els.sourcePreview.hidden = !open;
    els.sourceToggle.setAttribute("aria-pressed", String(open));
  }

  els.sourceCopy.addEventListener("click", () => void copy());
  els.sourceToggle.addEventListener("click", () => setPreviewOpen(els.sourcePreview.hidden));
  els.sourceClose.addEventListener("click", onClose);

  return {
    show(nextBinName) {
      els.sourcePanel.hidden = !nextBinName;
      if (nextBinName === binName) {
        return;
      }
      binName = nextBinName;
      if (!binName) {
        requestSeq += 1;
        return;
      }
      els.sourceBin.textContent = binName;
      els.sourceCopy.disabled = true;
      setFeedback("");
      void load().catch(() => {});
    },
    copy,
  };
}
