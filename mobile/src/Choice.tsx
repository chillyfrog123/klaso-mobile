import { useRef } from "react";
export function Choice({
  label,
  value,
  options,
  onChange,
  close,
}: {
  label: string;
  value: string | number;
  options: { value: string | number; label: string }[];
  onChange: (value: string) => void;
  close: string;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  return (
    <>
      <button
        type="button"
        className="choice-trigger"
        aria-label={label}
        aria-haspopup="dialog"
        onClick={() => dialog.current?.showModal()}
      >
        {options.find((o) => String(o.value) === String(value))?.label}
        <span aria-hidden="true">⌄</span>
      </button>
      <dialog
        ref={dialog}
        className="choice"
        aria-label={label}
        onClick={(e) => {
          if (e.target === dialog.current) dialog.current.close();
        }}
      >
        <h2>{label}</h2>
        <div role="radiogroup" aria-label={label}>
          {options.map((o) => (
            <button
              type="button"
              role="radio"
              aria-checked={String(o.value) === String(value)}
              key={o.value}
              onClick={() => {
                onChange(String(o.value));
                dialog.current?.close();
              }}
            >
              <span>{o.label}</span>
              <span aria-hidden="true">
                {String(o.value) === String(value) ? "●" : "○"}
              </span>
            </button>
          ))}
        </div>
        <button
          type="button"
          className="text-button"
          onClick={() => dialog.current?.close()}
        >
          {close}
        </button>
      </dialog>
    </>
  );
}
