import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
  /** Shown in the message, e.g. "Agent". */
  name: string;
  children: ReactNode;
}

interface State {
  error: Error | null;
}

/**
 * Keeps a crash inside one pane: shows the error with "Try again" instead of
 * unmounting the whole app (which leaves a blank window).
 */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error(`[${this.props.name}]`, error, info.componentStack);
  }

  render() {
    const { error } = this.state;
    if (!error) return this.props.children;
    const details = `${this.props.name}: ${error.message}\n${error.stack ?? ""}`;
    return (
      <div className="flex h-full min-h-0 flex-col gap-2 overflow-auto p-3 text-xs">
        <div className="font-medium text-bad">{this.props.name} hit an error</div>
        <pre className="max-h-48 overflow-auto whitespace-pre-wrap rounded-md border bg-bg p-2 font-mono text-2xs text-muted">
          {details}
        </pre>
        <div className="flex gap-2">
          <button
            onClick={() => this.setState({ error: null })}
            className="ease rounded-md border px-2.5 py-1 hover:border-line-strong"
          >
            Try again
          </button>
          <button
            onClick={() => navigator.clipboard?.writeText(details)}
            className="ease rounded-md border px-2.5 py-1 text-muted hover:border-line-strong"
          >
            Copy details
          </button>
        </div>
      </div>
    );
  }
}
