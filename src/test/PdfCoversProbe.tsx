import { usePdfCovers } from "../lib/pdfCover";

/** Mounts what App mounts once, so a test of one overlay sees what Rust is told about the preview. */
export function PdfCoversProbe() {
  usePdfCovers();
  return null;
}
