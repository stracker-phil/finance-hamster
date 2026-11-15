import { Component, type ErrorInfo, type ReactNode } from "react";
import { api } from "../api";
import { t } from "../i18n";

interface State {
  error: Error | null;
}

export class ErrorBoundary extends Component<{ children: ReactNode }, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    void api.logFrontendError(`UI crash: ${error.message}\n${info.componentStack ?? ""}`).catch(() => {});
  }

  render() {
    if (!this.state.error) return this.props.children;
    return (
      <div className="crash" role="alert">
        <h2>{t().components.somethingWrong}</h2>
        <pre>{this.state.error.message}</pre>
        <button className="btn btn--primary" onClick={() => this.setState({ error: null })}>
          {t().components.tryAgain}
        </button>
      </div>
    );
  }
}
