import { renderSlide } from "./deck.js";
import { loadFixture } from "./media.js";

const { invoke } = window.__TAURI__.core;
const slide = document.getElementById("slide");
const video = document.getElementById("video");
const mask = document.getElementById("mask");
let revision = -1;
let playingVideo = false;
let videoFailed = false;
video.onerror = () => { videoFailed = true; };

function paint(frame) {
  const showVideo = frame?.video === true;
  if (showVideo !== playingVideo) {
    playingVideo = showVideo;
    if (showVideo) {
      video.currentTime = 0;
      video.play().catch(() => { if (playingVideo) videoFailed = true; });
    } else {
      video.pause();
    }
  }
  video.hidden = !showVideo;
  slide.hidden = frame?.slide == null;
  if (!slide.hidden) renderSlide(slide, frame.slide);
  mask.hidden = !!frame && !frame.black && !(showVideo && videoFailed);
}

async function poll() {
  try {
    const frame = await invoke("projector", {
      videoReady: !videoFailed && video.readyState >= HTMLMediaElement.HAVE_CURRENT_DATA,
      videoFailed,
    });
    if (!frame || frame.revision !== revision) {
      revision = frame?.revision ?? -1;
      paint(frame);
    }
  } catch {
    revision = -1;
    paint(null);
  } finally {
    setTimeout(poll, 100);
  }
}

void poll();

loadFixture(video, "candidate.mp4", "video/mp4").catch(() => { videoFailed = true; });
