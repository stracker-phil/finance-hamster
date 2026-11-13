import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

export const DOCUMENT_KEY = ["document"] as const;
export const DOCUMENT_LINES_KEY = ["document-line-options"] as const;
export const DOCUMENTS_KEY = ["documents"] as const;
export const VENDORS_KEY = ["vendors"] as const;
export const HISTORY_KEY = ["document-history"] as const;
export const LOG_KEY = ["year-log"] as const;
export const ARCHIVE_KEY = ["archive"] as const;
export const DELETIONS_KEY = ["deletions"] as const;

export const useDocument = (id: string, enabled = true) => useQuery({ queryKey: [...DOCUMENT_KEY, id], queryFn: () => api.documentView(id), enabled, meta: { quiet: true } });

export const useDocumentLineOptions = (id: string, query: string, enabled: boolean) =>
  useQuery({ queryKey: [...DOCUMENT_LINES_KEY, id, query], queryFn: () => api.documentLineOptions(id, query), enabled });

export const useDocuments = (query: string) => useQuery({ queryKey: [...DOCUMENTS_KEY, query], queryFn: () => api.documentsList(query), placeholderData: (previous) => previous });

export const useVendors = () => useQuery({ queryKey: VENDORS_KEY, queryFn: api.vendorsView });

export const useDocumentHistory = (id: string) => useQuery({ queryKey: [...HISTORY_KEY, id], queryFn: () => api.documentHistory(id) });

export const useYearLog = (year: number, query = "") => useQuery({ queryKey: [...LOG_KEY, year, query], queryFn: () => api.yearLog(year, query), placeholderData: (previous) => previous });

export const useArchive = (year: number) => useQuery({ queryKey: [...ARCHIVE_KEY, year], queryFn: () => api.archiveList(year) });

export const useDeletions = () => useQuery({ queryKey: DELETIONS_KEY, queryFn: api.deletionLog });
