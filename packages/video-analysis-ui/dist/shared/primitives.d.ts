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
export interface DataTableColumn<T> {
    key: string;
    header: ReactNode;
    cell: (row: T, index: number) => ReactNode;
    className?: string;
    headerClassName?: string;
}
export declare function DataTable<T>({ rows, columns, getRowKey, empty, onRowClick, rowClassName, }: {
    rows: T[];
    columns: Array<DataTableColumn<T>>;
    getRowKey: (row: T, index: number) => string;
    empty?: ReactNode;
    onRowClick?: (row: T, index: number) => void;
    rowClassName?: (row: T, index: number) => string | false | null | undefined;
}): import("react/jsx-runtime").JSX.Element;
export declare function ScoreMeter({ value }: {
    value?: number | null;
}): import("react/jsx-runtime").JSX.Element;
//# sourceMappingURL=primitives.d.ts.map