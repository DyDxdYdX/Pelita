export const slides = [
  { title: "Dawn at the harbour", color: "#152e4a" },
  { title: "Rain on the footpath", color: "#265044" },
  { title: "Lanterns on the hill", color: "#583b58" },
];

export function renderSlide(element, index) {
  element.textContent = slides[index].title;
  element.style.backgroundColor = slides[index].color;
}
