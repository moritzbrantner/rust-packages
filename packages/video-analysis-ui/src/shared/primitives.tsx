import type { KeyboardEvent, ReactNode } from "react";
import {
  Badge as UiBadge,
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  Empty,
  EmptyDescription,
  EmptyHeader,
  Stat,
  StatDescription,
  StatLabel,
  StatValue,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@moritzbrantner/ui";

import { cn } from "./utils";

export type Tone = "neutral" | "sky" | "emerald" | "amber" | "rose" | "violet";

const toneClasses: Record<Tone, string> = {
  neutral: "border-zinc-200 bg-white text-zinc-700",
  sky: "border-sky-200 bg-sky-50 text-sky-800",
  emerald: "border-emerald-200 bg-emerald-50 text-emerald-800",
  amber: "border-amber-200 bg-amber-50 text-amber-800",
  rose: "border-rose-200 bg-rose-50 text-rose-800",
  violet: "border-violet-200 bg-violet-50 text-violet-800",
};

export function Panel({
  title,
  description,
  actions,
  children,
  className,
}: {
  title?: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <Card
      className={cn(
        "gap-0 rounded-lg border border-zinc-200 bg-white py-0 text-zinc-950 shadow-sm ring-0",
        className,
      )}
    >
      {(title || description || actions) && (
        <CardHeader className="flex flex-col gap-3 border-b border-zinc-200 px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
          <div>
            {title && <h2 className="text-sm font-semibold text-zinc-950">{title}</h2>}
            {description && (
              <CardDescription className="mt-1 text-sm text-zinc-600">
                {description}
              </CardDescription>
            )}
          </div>
          {actions && <CardAction className="flex items-center gap-2">{actions}</CardAction>}
        </CardHeader>
      )}
      <CardContent className="p-4">{children}</CardContent>
    </Card>
  );
}

export function Badge({
  children,
  tone = "neutral",
  className,
}: {
  children: ReactNode;
  tone?: Tone;
  className?: string;
}) {
  return (
    <UiBadge
      variant="outline"
      className={cn(
        "inline-flex min-h-6 rounded-md border px-2 py-0.5 text-xs font-medium shadow-none",
        toneClasses[tone],
        className,
      )}
    >
      {children}
    </UiBadge>
  );
}

export function StatCard({
  label,
  value,
  detail,
  tone = "neutral",
}: {
  label: ReactNode;
  value: ReactNode;
  detail?: ReactNode;
  tone?: Tone;
}) {
  return (
    <Stat className={cn("gap-1 rounded-lg border p-3 shadow-none", toneClasses[tone])}>
      <StatLabel className="text-xs font-medium uppercase tracking-normal opacity-75">
        {label}
      </StatLabel>
      <StatValue className="mt-1 text-xl font-semibold text-zinc-950">{value}</StatValue>
      {detail && (
        <StatDescription className="mt-1 text-xs leading-normal opacity-75">
          {detail}
        </StatDescription>
      )}
    </Stat>
  );
}

export function EmptyState({ children = "No results" }: { children?: ReactNode }) {
  return (
    <Empty className="min-h-24 rounded-lg border border-dashed border-zinc-300 bg-zinc-50 px-4 py-6 text-sm text-zinc-500">
      <EmptyHeader>
        <EmptyDescription className="text-sm text-zinc-500">{children}</EmptyDescription>
      </EmptyHeader>
    </Empty>
  );
}

export interface DataTableColumn<T> {
  key: string;
  header: ReactNode;
  cell: (row: T, index: number) => ReactNode;
  className?: string;
  headerClassName?: string;
}

export function DataTable<T>({
  rows,
  columns,
  getRowKey,
  empty = "No rows",
  onRowClick,
  rowClassName,
}: {
  rows: T[];
  columns: Array<DataTableColumn<T>>;
  getRowKey: (row: T, index: number) => string;
  empty?: ReactNode;
  onRowClick?: (row: T, index: number) => void;
  rowClassName?: (row: T, index: number) => string | false | null | undefined;
}) {
  if (rows.length === 0) {
    return <EmptyState>{empty}</EmptyState>;
  }

  const handleRowKeyDown = (event: KeyboardEvent<HTMLTableRowElement>, row: T, index: number) => {
    if (!onRowClick || (event.key !== "Enter" && event.key !== " ")) {
      return;
    }
    event.preventDefault();
    onRowClick(row, index);
  };

  return (
    <Table className="min-w-full text-left text-sm">
      <TableHeader className="border-b border-zinc-200 text-xs uppercase text-zinc-500">
        <TableRow className="border-zinc-200 hover:bg-transparent">
          {columns.map((column) => (
            <TableHead
              key={column.key}
              className={cn("px-3 py-2 font-medium text-zinc-500", column.headerClassName)}
            >
              {column.header}
            </TableHead>
          ))}
        </TableRow>
      </TableHeader>
      <TableBody className="divide-y divide-zinc-100">
        {rows.map((row, index) => (
          <TableRow
            key={getRowKey(row, index)}
            className={cn(
              "border-zinc-100 hover:bg-zinc-50",
              onRowClick && "cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sky-400",
              rowClassName?.(row, index),
            )}
            tabIndex={onRowClick ? 0 : undefined}
            role={onRowClick ? "button" : undefined}
            onClick={() => onRowClick?.(row, index)}
            onKeyDown={(event) => handleRowKeyDown(event, row, index)}
          >
            {columns.map((column) => (
              <TableCell key={column.key} className={cn("px-3 py-2", column.className)}>
                {column.cell(row, index)}
              </TableCell>
            ))}
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}

export function ScoreMeter({ value }: { value?: number | null }) {
  const normalized = value == null ? 0 : value <= 1 ? value * 100 : Math.min(value, 100);
  return (
    <div className="flex min-w-28 items-center gap-2">
      <div className="h-2 w-20 overflow-hidden rounded-full bg-zinc-200">
        <div className="h-full rounded-full bg-emerald-500" style={{ width: `${normalized}%` }} />
      </div>
      <span className="w-12 text-right text-xs tabular-nums text-zinc-600">
        {value == null ? "n/a" : value <= 1 ? `${Math.round(value * 100)}%` : value.toFixed(1)}
      </span>
    </div>
  );
}
