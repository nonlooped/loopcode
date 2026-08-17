import type { ComposerImage } from "../types/index.ts";

export const MAX_COMPOSER_IMAGES = 8;
export const MAX_COMPOSER_IMAGE_BYTES = 20 * 1024 * 1024;

export interface AttachmentSelection {
  images: ComposerImage[];
  error: string;
}

export async function loadComposerImages(
  files: File[],
  existingCount: number,
): Promise<AttachmentSelection> {
  const imageFiles = files.filter((file) => file.type.startsWith("image/"));
  if (imageFiles.length === 0) return { images: [], error: "" };

  const available = Math.max(0, MAX_COMPOSER_IMAGES - existingCount);
  const withinSizeLimit = imageFiles.filter((file) => file.size <= MAX_COMPOSER_IMAGE_BYTES);
  const accepted = withinSizeLimit.slice(0, available);
  const skippedForSize = imageFiles.length - withinSizeLimit.length;
  const skippedForCount = withinSizeLimit.length - accepted.length;
  const error =
    skippedForSize > 0
      ? `Images must be smaller than ${MAX_COMPOSER_IMAGE_BYTES / 1024 / 1024} MB.`
      : skippedForCount > 0 || available === 0
        ? `You can attach up to ${MAX_COMPOSER_IMAGES} images.`
        : "";
  return {
    images: await Promise.all(accepted.map(readComposerImage)),
    error,
  };
}

function readComposerImage(file: File) {
  return new Promise<ComposerImage>((resolve, reject) => {
    const reader = new FileReader();
    const name = file.name || "Pasted image";
    reader.onerror = () => reject(new Error(`Could not read ${name}.`));
    reader.onload = () => {
      if (typeof reader.result !== "string") {
        reject(new Error(`Could not read ${name}.`));
        return;
      }
      const separator = reader.result.indexOf(",");
      if (separator < 0) {
        reject(new Error(`Could not read ${name}.`));
        return;
      }
      resolve({
        id: crypto.randomUUID(),
        type: "image",
        data: reader.result.slice(separator + 1),
        mimeType: file.type || "image/png",
        name,
        previewUrl: reader.result,
      });
    };
    reader.readAsDataURL(file);
  });
}
