import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

export const DOCUMENT_KEY = ["document"] as const;
export const DOCUMENT_LINES_KEY = ["document-line-options"] as const;
export const DOCUMENTS_KEY = ["documents"] as const;
export const VENDORS_KEY = ["vendors"] as const;

export const useDocument = (id: string, enabled = true) => useQuery({ queryKey: [...DOCUMENT_KEY, id], queryFn: () => api.documentView(id), retry: false, enabled });

export const useDocumentLineOptions = (id: string, query: string, enabled: boolean) =>
  useQuery({ queryKey: [...DOCUMENT_LINES_KEY, id, query], queryFn: () => api.documentLineOptions(id, query), enabled });

export const useDocuments = (query: string) => useQuery({ queryKey: [...DOCUMENTS_KEY, query], queryFn: () => api.documentsList(query), placeholderData: (previous) => previous });

export const useVendors = () => useQuery({ queryKey: VENDORS_KEY, queryFn: api.vendorsView });
