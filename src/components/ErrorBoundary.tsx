import { Component, type ReactNode } from "react";

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error: string;
}

export default class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: "" };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error: `${error.name}: ${error.message}\n${error.stack}` };
  }

  render() {
    if (this.state.hasError) {
      return (
        <div style={{
          padding: 20, color: "#e94560", background: "#1a1a2e",
          fontFamily: "monospace", fontSize: 12, whiteSpace: "pre-wrap",
          overflow: "auto", height: "100%"
        }}>
          <h3 style={{ color: "#fff" }}>應用程式錯誤</h3>
          <p>{this.state.error}</p>
          <button
            onClick={() => this.setState({ hasError: false, error: "" })}
            style={{ marginTop: 10, padding: "6px 12px", cursor: "pointer" }}
          >
            重試
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
