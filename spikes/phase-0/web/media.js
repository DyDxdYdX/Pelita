// The fixture is small enough to read once. The webview can seek the Blob
// without requiring byte-range support from Tauri's bundled-page protocol.
export async function loadFixture(element, name, mimeType) {
  const response = await fetch(`fixtures/${name}`);
  if (!response.ok) throw new Error(`Could not load ${name}.`);
  const bytes = await response.arrayBuffer();
  element.src = URL.createObjectURL(new Blob([bytes], { type: mimeType }));
}
