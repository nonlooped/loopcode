import { deleteAttachment, storeAttachment } from "../services/native.ts";
import type { ComposerImage } from "../types/index.ts";
import { isObject } from "./json.ts";

export const MAX_COMPOSER_IMAGES = 8;
export const MAX_COMPOSER_IMAGE_BYTES = 20 * 1024 * 1024;
const MAX_BASE64_LENGTH = Math.ceil(MAX_COMPOSER_IMAGE_BYTES / 3) * 4;

type StoreAttachment = (attachmentId: string, bytes: Uint8Array) => Promise<void>;
type DeleteAttachment = (attachmentId: string) => Promise<void>;

export interface AttachmentSelection {
  images: ComposerImage[];
  error: string;
}

export async function loadComposerImages(
  files: File[],
  existingCount: number,
  store: StoreAttachment = storeAttachment,
  remove: DeleteAttachment = deleteAttachment,
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
  const images: ComposerImage[] = [];
  try {
    for (const file of accepted) {
      const attachmentId = crypto.randomUUID();
      await store(attachmentId, new Uint8Array(await file.arrayBuffer()));
      let previewUrl: string;
      try {
        previewUrl = URL.createObjectURL(file);
      } catch (cause) {
        await remove(attachmentId).catch(() => {});
        throw cause;
      }
      images.push({
        attachmentId,
        type: "image",
        mimeType: file.type || "image/png",
        name: file.name || "Pasted image",
        previewUrl,
      });
    }
    return { images, error };
  } catch (cause) {
    await disposeComposerImages(images, remove);
    throw cause;
  }
}

export async function disposeComposerImages(
  images: ComposerImage[],
  remove: DeleteAttachment = deleteAttachment,
) {
  for (const image of images) URL.revokeObjectURL(image.previewUrl);
  await Promise.allSettled(images.map((image) => remove(image.attachmentId)));
}

export interface MigrationResult {
  migrated: number;
  failed: number;
}

export async function migrateLegacyAttachments(
  workspace: unknown,
  store: StoreAttachment = storeAttachment,
  createId: () => string = () => crypto.randomUUID(),
): Promise<MigrationResult> {
  const result = { migrated: 0, failed: 0 };
  if (!isObject(workspace) || !Array.isArray(workspace.threads)) return result;
  for (const thread of workspace.threads) {
    if (!isObject(thread) || !Array.isArray(thread.messages)) continue;
    for (const message of thread.messages) {
      if (!isObject(message) || !Array.isArray(message.images)) continue;
      for (let index = 0; index < message.images.length; index += 1) {
        const image = message.images[index];
        if (!isObject(image) || typeof image.attachmentId === "string") continue;
        const data = typeof image.data === "string" ? image.data : "";
        const mimeType = typeof image.mimeType === "string" ? image.mimeType : "";
        if (!mimeType.match(/^image\/[a-z0-9.+-]+$/i)) continue;
        const bytes = decodeBoundedBase64(data);
        if (!bytes) {
          result.failed += 1;
          continue;
        }
        const attachmentId = createId();
        try {
          await store(attachmentId, bytes);
          message.images[index] = {
            attachmentId,
            mimeType,
            name: typeof image.name === "string" && image.name ? image.name : "Attached image",
          };
          result.migrated += 1;
        } catch {
          // The inline record remains the durable copy when this individual write fails.
          result.failed += 1;
        }
      }
    }
  }
  return result;
}

function decodeBoundedBase64(data: string): Uint8Array | undefined {
  if (
    !data ||
    data.length > MAX_BASE64_LENGTH ||
    data.length % 4 !== 0 ||
    !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(data)
  ) {
    return undefined;
  }
  const padding = data.endsWith("==") ? 2 : data.endsWith("=") ? 1 : 0;
  if ((data.length / 4) * 3 - padding > MAX_COMPOSER_IMAGE_BYTES) return undefined;
  try {
    const binary = atob(data);
    return Uint8Array.from(binary, (character) => character.charCodeAt(0));
  } catch {
    return undefined;
  }
}
