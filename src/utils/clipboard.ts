export function copyText(text: string): Promise<void> {
  return navigator.clipboard.writeText(text);
}

export async function copyImage(src: string): Promise<void> {
  const blob = await fetch(src).then((response) => response.blob());
  await navigator.clipboard.write([new ClipboardItem({ [blob.type]: blob })]);
}

export function saveImage(src: string, name: string): void {
  const link = document.createElement("a");
  link.href = src;
  link.download = name || "image";
  link.click();
}
