import { renderSlide, slides } from "./deck.js";
import { loadFixture } from "./media.js";

const { invoke } = window.__TAURI__.core;
const element = (id) => document.getElementById(id);
const displaySelect = element("displays");
const preview = element("preview");
const audio = element("audio");
let topology = [];
let busy = false;
let videoLive = false;

slides.forEach(({ title }, index) => preview.add(new Option(title, index)));
const showPreview = () => renderSlide(element("preview-frame"), Number(preview.value));
preview.addEventListener("change", showPreview);
showPreview();

async function refresh() {
  topology = await invoke("displays");
  displaySelect.replaceChildren(new Option("Select an extended target", ""));
  topology.forEach(({ name, width, height, x, y, scale }, index) => {
    displaySelect.add(new Option(`${name || "Display"} · ${width}×${height} @ ${x},${y} · scale ${scale}`, index));
  });
}

async function run(callback) {
  if (busy) return;
  busy = true;
  element("error").textContent = "";
  try {
    await callback();
  } catch (error) {
    element("error").textContent = String(error);
  } finally {
    busy = false;
  }
}

element("refresh").onclick = () => run(refresh);
element("confirm").onclick = () => run(async () => {
  if (!displaySelect.value) throw new Error("Select an extended target.");
  await invoke("confirm", { topology, index: Number(displaySelect.value) });
});

audio.addEventListener("play", () => {
  if (videoLive) audio.pause();
});
audio.onerror = () => { element("error").textContent = "Accompaniment failed to load."; };
element("restart").onclick = () => run(async () => {
  audio.currentTime = 0;
  await audio.play();
});

element("actions").onclick = (event) => {
  const intent = event.target.dataset.action;
  if (!intent) return;
  void run(async () => {
    if (intent === "video") {
      const current = await invoke("status");
      if (!current.video_ready) throw new Error("Candidate video has not decoded a frame.");
      // Pause before releasing video to the projector. Undo the pause if Take is rejected.
      const resume = !audio.paused;
      videoLive = true;
      audio.pause();
      try {
        await invoke("action", { intent, slide: 0 });
      } catch (error) {
        videoLive = false;
        if (resume) await audio.play();
        throw error;
      }
      audio.currentTime = 0;
    } else {
      await invoke("action", { intent, slide: Number(preview.value) });
      if (intent === "clear") {
        audio.pause();
        audio.currentTime = 0;
      }
    }
  });
};

async function poll() {
  try {
    const { message, frame, ready, video_ready } = await invoke("status");
    const status = message || "Projection idle";
    if (element("status").textContent !== status) element("status").textContent = status;
    if (!busy) videoLive = frame?.video === true;
    element("take-video").disabled = !video_ready || videoLive;
    const label = frame?.video ? "Candidate video" : slides[frame?.slide]?.title ?? "No live frame";
    element("live").textContent = ready ? `${label}${frame.black ? " (black)" : ""}` : "No confirmed output";
  } catch (error) {
    element("error").textContent = String(error);
  } finally {
    setTimeout(poll, 250);
  }
}

void run(refresh);
void poll();

loadFixture(audio, "continuous.mp3", "audio/mpeg").catch((error) => {
  element("error").textContent = String(error);
});
