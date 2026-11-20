import { useEffect, useRef, useState } from "react";
import { registerPlugin } from "@capacitor/core";
import { Copy, Share2, TextSelect } from "lucide-react";
import { native } from "./bridge";
import type { Label } from "./i18n";
import "./text-selection.css";

const actions = registerPlugin<{
  copy(options: { text: string; label: string }): Promise<void>;
  share(options: { text: string; title: string }): Promise<void>;
}>("TextActions");

const editable =
  "input, textarea, select, [contenteditable]:not([contenteditable='false'])";
const interactive = `${editable}, button, a, dialog`;
const elementOf = (node: Node | null) =>
  node instanceof Element ? node : node?.parentElement;

function captureSelection() {
  if (document.activeElement?.closest(editable)) return null;
  const selection = window.getSelection();
  if (!selection || selection.isCollapsed || !selection.rangeCount) return null;
  const range = selection.getRangeAt(0);
  const start = elementOf(range.startContainer);
  const end = elementOf(range.endContainer);
  if (start?.closest(interactive) || end?.closest(interactive)) return null;
  const scope = elementOf(range.commonAncestorContainer)?.closest<HTMLElement>(
    ".message-body, .reader, .notice, .card, main",
  );
  if (!scope?.closest("main")) return null;
  const text = selection.toString();
  const rect = range.getBoundingClientRect();
  const position = {
    left: Math.max(112, Math.min(innerWidth - 112, rect.left + rect.width / 2)),
    top: Math.max(
      8,
      Math.min(
        innerHeight - 60,
        rect.top >= 64 ? rect.top - 56 : rect.bottom + 8,
      ),
    ),
  };
  return text.trim() ? { text, allText: scope.innerText, position } : null;
}

export function dismissTextSelection() {
  const event = new Event("klaso-dismiss-text-selection", { cancelable: true });
  document.dispatchEvent(event);
  return event.defaultPrevented;
}

export function TextSelection({ t }: { t: (key: Label) => string }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const generation = useRef(0);
  const nativeSelection = useRef(false);
  const [trigger, setTrigger] = useState<{ left: number; top: number } | null>(
    null,
  );
  const [selection, setSelection] =
    useState<ReturnType<typeof captureSelection>>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<Label | null>(null);

  function showActions(captured: ReturnType<typeof captureSelection>) {
    if (!captured || !dialog.current) return false;
    if (document.querySelector("dialog[open]")) return false;
    generation.current++;
    nativeSelection.current = false;
    setTrigger(null);
    setSelection(captured);
    setError(null);
    setBusy(false);
    dialog.current.showModal();
    window.getSelection()?.removeAllRanges();
    return true;
  }

  useEffect(() => {
    const requestNative = (event: Event) => {
      if (dialog.current?.open) {
        event.preventDefault();
        return;
      }
      if (document.querySelector("dialog[open]")) return;
      const captured = captureSelection();
      if (!captured) return;
      // Keep the native handles available until the user opens the actions.
      nativeSelection.current = true;
      setTrigger(captured.position);
      event.preventDefault();
    };
    const updateTrigger = () => {
      if (nativeSelection.current)
        setTrigger(captureSelection()?.position ?? null);
    };
    const endNative = () => {
      nativeSelection.current = false;
      setTrigger(null);
    };
    const dismissNative = (event: Event) => {
      if (!nativeSelection.current) return;
      event.preventDefault();
      endNative();
      window.getSelection()?.removeAllRanges();
    };
    const contextMenu = (event: MouseEvent) => {
      // WebView must first establish its selection. MainActivity then asks us
      // to capture it; canceling its initial long-press would lose that text.
      if (native || elementOf(event.target as Node)?.closest(interactive))
        return;
      if (showActions(captureSelection())) event.preventDefault();
    };
    document.addEventListener("contextmenu", contextMenu);
    document.addEventListener("klaso-text-selection", requestNative);
    document.addEventListener("klaso-text-selection-ended", endNative);
    document.addEventListener("klaso-dismiss-text-selection", dismissNative);
    document.addEventListener("selectionchange", updateTrigger);
    document.addEventListener("scroll", updateTrigger, true);
    window.addEventListener("resize", updateTrigger);
    return () => {
      document.removeEventListener("contextmenu", contextMenu);
      document.removeEventListener("klaso-text-selection", requestNative);
      document.removeEventListener("klaso-text-selection-ended", endNative);
      document.removeEventListener(
        "klaso-dismiss-text-selection",
        dismissNative,
      );
      document.removeEventListener("selectionchange", updateTrigger);
      document.removeEventListener("scroll", updateTrigger, true);
      window.removeEventListener("resize", updateTrigger);
    };
  }, []);

  async function perform(action: "copy" | "share") {
    if (!selection || busy) return;
    const current = generation.current;
    setBusy(true);
    setError(null);
    try {
      if (action === "copy") {
        if (native)
          await actions.copy({
            text: selection.text,
            label: t("selectedText"),
          });
        else await navigator.clipboard.writeText(selection.text);
      } else {
        if (native)
          await actions.share({ text: selection.text, title: t("share") });
        else await navigator.share({ text: selection.text });
      }
      if (generation.current === current) dialog.current?.close();
    } catch (failure) {
      if (generation.current !== current) return;
      // Canceling the system share sheet is a normal dismissal.
      if ((failure as { name?: string })?.name === "AbortError") return;
      setError(action === "copy" ? "copyFailed" : "shareFailed");
    } finally {
      if (generation.current === current) setBusy(false);
    }
  }

  return (
    <>
      {trigger && (
        <div className="selection-anchor" style={trigger}>
          <button
            type="button"
            className="choice-trigger selection-trigger"
            aria-label={t("selectedText")}
            aria-haspopup="dialog"
            onPointerDown={(event) => event.preventDefault()}
            onClick={() => showActions(captureSelection())}
          >
            {t("selectedText")}
            <span aria-hidden="true">⌄</span>
          </button>
        </div>
      )}
      <dialog
        ref={dialog}
        className="choice text-selection"
        aria-label={t("selectedText")}
        onClose={() => {
          if (dialog.current?.open) return;
          generation.current++;
          setSelection(null);
          setError(null);
          setBusy(false);
        }}
        onClick={(event) => {
          if (event.target === dialog.current) dialog.current.close();
        }}
      >
        <h2>{t("selectedText")}</h2>
        <p className="selection-preview">{selection?.text}</p>
        {error && <p role="alert">{t(error)}</p>}
        <button
          type="button"
          className="selection-action"
          disabled={busy}
          onClick={() => void perform("copy")}
        >
          <Copy size={18} aria-hidden="true" />
          {t("copy")}
        </button>
        <button
          type="button"
          className="selection-action"
          disabled={busy}
          onClick={() =>
            setSelection((previous) =>
              previous ? { ...previous, text: previous.allText } : previous,
            )
          }
        >
          <TextSelect size={18} aria-hidden="true" />
          {t("selectAll")}
        </button>
        {(native || typeof navigator.share === "function") && (
          <button
            type="button"
            className="selection-action"
            disabled={busy}
            onClick={() => void perform("share")}
          >
            <Share2 size={18} aria-hidden="true" />
            {t("share")}
          </button>
        )}
        <button
          type="button"
          className="text-button"
          onClick={() => dialog.current?.close()}
        >
          {t("close")}
        </button>
      </dialog>
    </>
  );
}
