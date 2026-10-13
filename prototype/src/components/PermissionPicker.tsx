import { useEffect, useRef } from "react";
import {
  CheckIcon,
  HandPalmIcon,
  ShieldCheckIcon,
  ShieldWarningIcon,
} from "@phosphor-icons/react";
import type { PermissionMode } from "../bridge";
import shieldCode from "../assets/codex/composer_shield_code-703ef9151d94.svg";
import shieldExclamation from "../assets/codex/composer_shield_exclamation-fcbfc0396226.svg";

type PermissionPickerProps = {
  value: PermissionMode;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onChange: (value: PermissionMode) => void;
};
const permissionOptions: {
  id: PermissionMode;
  title: string;
  detail: string;
  icon: typeof HandPalmIcon;
}[] = [
  {
    id: "ask",
    title: "请求批准",
    detail: "编辑外部文件和使用互联网时始终询问",
    icon: HandPalmIcon,
  },
  {
    id: "auto",
    title: "帮我批准",
    detail: "仅对检测到的风险操作请求批准",
    icon: ShieldCheckIcon,
  },
  {
    id: "full",
    title: "完全访问权限",
    detail: "可不受限制地访问互联网和你电脑上的任何文件",
    icon: ShieldWarningIcon,
  },
];

export function PermissionPicker({
  value,
  open,
  onOpenChange,
  onChange,
}: PermissionPickerProps) {
  const selected =
    permissionOptions.find((item) => item.id === value) || permissionOptions[0];
  const root = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const close = (event: MouseEvent) => {
      if (root.current && !root.current.contains(event.target as Node))
        onOpenChange(false);
    };
    document.addEventListener("mousedown", close);
    return () => document.removeEventListener("mousedown", close);
  }, [onOpenChange]);
  return (
    <div className="permission-picker" ref={root}>
      {open && (
        <div className="permission-popover" role="menu">
          <div className="permission-popover-heading">
            <strong>应如何批准 ChatGPT 操作?</strong>
            <a
              href="https://developers.openai.com/codex/security/"
              target="_blank"
              rel="noreferrer"
            >
              了解更多
            </a>
          </div>
          {permissionOptions.map((option) => {
            const Icon = option.icon;
            return (
              <button
                type="button"
                role="menuitemradio"
                aria-checked={option.id === value}
                key={option.id}
                className={`permission-option${option.id === value ? " is-current" : ""}`}
                onClick={() => {
                  onChange(option.id);
                  onOpenChange(false);
                }}
              >
                <Icon
                  size={23}
                  weight={option.id === "full" ? "regular" : "bold"}
                />
                <span>
                  <strong>{option.title}</strong>
                  <small>{option.detail}</small>
                </span>
                {option.id === value && <CheckIcon size={19} weight="bold" />}
              </button>
            );
          })}
        </div>
      )}
      <button
        type="button"
        className={`permission-trigger is-${value}`}
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => onOpenChange(!open)}
      >
        <img src={value === "full" ? shieldExclamation : shieldCode} alt="" />
        <span>{selected.title}</span>
      </button>
    </div>
  );
}
