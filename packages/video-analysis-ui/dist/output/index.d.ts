import { type ReactNode } from "react";
export declare function ReportShell({ title, subtitle, actions, children, }: {
    title?: ReactNode;
    subtitle?: ReactNode;
    actions?: ReactNode;
    children: ReactNode;
}): import("react/jsx-runtime").JSX.Element;
export declare function JsonReportLoader<T>({ onLoad, label, }: {
    onLoad: (report: T) => void;
    label?: string;
}): import("react/jsx-runtime").JSX.Element;
//# sourceMappingURL=index.d.ts.map