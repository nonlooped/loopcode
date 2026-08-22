export function copyText(text: string): Promise<void> {
  return navigator.clipboard.writeText(text);
}

export async function copyImage(src: string): Promise<void> {
  if (!navigator.clipboard?.write || typeof ClipboardItem === "undefined") {
    throw new Error("Copying images is not supported");
  }
  const blob = await fetch(src).then((response) => response.blob());
  const type = blob.type || "image/png";
  if (!type.startsWith("image/")) throw new Error("Clipboard source is not an image");
  await navigator.clipboard.write([new ClipboardItem({ [type]: blob })]);
}

export function saveImage(src: string, name: string): void {
  const link = document.createElement("a");
  link.href = src;
  link.download = name || "image";
  link.click();
}
