import { Fragment, type ReactNode } from "react";

/** fill() の React 版。"{code}" などを要素に置き換える */
export function fillNode(template: string, values: Record<string, ReactNode>): ReactNode {
  return template.split(/(\{\w+\})/g).map((chunk, i) => {
    const key = /^\{(\w+)\}$/.exec(chunk)?.[1];
    return <Fragment key={i}>{key !== undefined && key in values ? values[key] : chunk}</Fragment>;
  });
}
