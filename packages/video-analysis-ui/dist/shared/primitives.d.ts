import type { ReactNode } from "react";
export type Tone = "neutral" | "sky" | "emerald" | "amber" | "rose" | "violet";
export declare function Panel({ title, description, actions, children, className, }: {
    title?: ReactNode;
    description?: ReactNode;
    actions?: ReactNode;
    children: ReactNode;
    className?: string;
}): import("react/jsx-runtime").JSX.Element;
export declare function Badge({ children, tone, className, }: {
    children: ReactNode;
    tone?: Tone;
    className?: string;
}): import("react/jsx-runtime").JSX.Element;
export declare function StatCard({ label, value, detail, tone, }: {
    label: ReactNode;
    value: ReactNode;
    detail?: ReactNode;
    tone?: Tone;
}): import("react/jsx-runtime").JSX.Element;
export declare function EmptyState({ children }: {
    children?: ReactNode;
}): import("react/jsx-runtime").JSX.Element;
export declare function ScoreMeter({ value }: {
    value?: number | null;
}): import("react/jsx-runtime").JSX.Element;
//# sourceMappingURL=primitives.d.ts.map