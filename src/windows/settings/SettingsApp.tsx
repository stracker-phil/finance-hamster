import { MenuHost } from "../../components/menu";
import { useCallback, useEffect, useState } from "react";
import { QueryClientProvider, type QueryClient } from "@tanstack/react-query";
import { api } from "../../api";
import { ConfirmHost } from "../../components/ConfirmSheet";
import { Toasts } from "../../components/Toasts";
import { TooltipHost } from "../../components/TooltipHost";
import { queryClient as defaultClient } from "../../queries/queryClient";
import { useEvents } from "../../queries/useEvents";
import { useLook } from "../../queries/useLook";
import { SettingsWindow, type SettingsRequest } from "../../settings/SettingsWindow";

/** The page or row Rust opened the window at (`?page=`); later ones arrive as `open_settings`. */
function initialRequest(): SettingsRequest {
  return { id: new URLSearchParams(window.location.search).get("page") ?? undefined };
}

function SettingsRoot() {
  const [request, setRequest] = useState(initialRequest);
  const open = useCallback((id: string) => setRequest({ id }), []);
  useEffect(() => {
    const unlisten = api.listen("open_settings", open);
    return () => void unlisten.then((stop) => stop()).catch(() => {});
  }, [open]);
  useEvents();
  useLook();
  return (
    <>
      <SettingsWindow request={request} onOpen={open} />
      <Toasts />
      <ConfirmHost />
      <MenuHost />
      <TooltipHost />
    </>
  );
}

/** The Settings window's page; tests pass a fresh query client. */
export function SettingsApp({ client = defaultClient }: { client?: QueryClient }) {
  return (
    <QueryClientProvider client={client}>
      <SettingsRoot />
    </QueryClientProvider>
  );
}
