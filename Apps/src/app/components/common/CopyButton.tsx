import { Copy, ShieldCheck } from "lucide-react";
import { useState } from "react";

type CopyButtonProps = {
  text: string;
};

export function CopyButton({ text }: CopyButtonProps) {
  const [copied, setCopied] = useState(false);

  return (
    <button
      onClick={() => {
        navigator.clipboard.writeText(text).catch(() => undefined);
        setCopied(true);
        window.setTimeout(() => setCopied(false), 1_600);
      }}
      className="rounded p-1.5 text-muted-foreground transition-colors hover:bg-primary/10 hover:text-primary"
      title="复制内容"
      type="button"
    >
      {copied ? <ShieldCheck size={13} /> : <Copy size={13} />}
    </button>
  );
}
