import { useAppEvents } from "./state/useAppEvents";
import { useAppInfo } from "./state/useAppInfo";
import { useStore } from "./state/store";

export default function App() {
  useAppEvents();
  useAppInfo();
  const appInfo = useStore((s) => s.appInfo);
  const launchError = useStore((s) => s.launchError);
  return (
    <main data-tauri-drag-region style={{ height: "100vh", display: "grid", placeItems: "center", font: "13px -apple-system, sans-serif" }}>
      <div>
        <h1 style={{ fontSize: 17, margin: 0 }}>{appInfo?.name ?? "Hamster"}</h1>
        {appInfo && <p>Version {appInfo.version}</p>}
        {launchError && <p role="alert">{launchError}</p>}
      </div>
    </main>
  );
}
