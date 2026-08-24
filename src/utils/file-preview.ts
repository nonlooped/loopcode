type DocumentType = "html" | "markdown";

export function documentTypeForPath(path: string): DocumentType | null {
  switch (path.split(".").pop()?.toLowerCase()) {
    case "htm":
    case "html":
      return "html";
    case "md":
    case "markdown":
      return "markdown";
    default:
      return null;
  }
}
