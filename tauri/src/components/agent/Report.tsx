import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";

/** The agent's final report, styled to match the workbench (no prose plugin). */
export function Report({ text }: { text: string }) {
  return (
    <div className="space-y-2 text-sm leading-relaxed text-fg">
      <Markdown
        remarkPlugins={[remarkGfm]}
        components={{
          h1: ({ children }) => <div className="label mt-3 text-fg">{children}</div>,
          h2: ({ children }) => <div className="label mt-3 text-fg">{children}</div>,
          h3: ({ children }) => <div className="mt-2 text-sm font-semibold">{children}</div>,
          p: ({ children }) => <p className="text-sm text-fg">{children}</p>,
          ul: ({ children }) => <ul className="list-disc space-y-0.5 pl-4">{children}</ul>,
          ol: ({ children }) => <ol className="list-decimal space-y-0.5 pl-4">{children}</ol>,
          code: ({ children }) => (
            <code className="rounded-sm bg-active px-1 font-mono text-xs text-asm-symbol">
              {children}
            </code>
          ),
          table: ({ children }) => (
            <div className="overflow-x-auto">
              <table className="w-full border-collapse text-xs">{children}</table>
            </div>
          ),
          th: ({ children }) => (
            <th className="border-b px-2 py-1 text-left font-semibold text-muted">{children}</th>
          ),
          td: ({ children }) => (
            <td className="border-b px-2 py-1 align-top font-mono">{children}</td>
          ),
          em: ({ children }) => <em className="text-muted">{children}</em>,
        }}
      >
        {text}
      </Markdown>
    </div>
  );
}
