import styles from './ProgressBar.module.css';

interface ProgressBarProps {
  /** 0 to 1, or `null` for work whose size is not known yet. */
  value: number | null;
  label: string;
}

export function ProgressBar({ value, label }: ProgressBarProps) {
  const determinate = value !== null && Number.isFinite(value);
  const percent = determinate ? Math.min(100, Math.max(0, value * 100)) : undefined;

  return (
    <div
      className={styles.track}
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={determinate ? Math.round(percent ?? 0) : undefined}
    >
      <div
        className={styles.fill}
        data-indeterminate={!determinate}
        style={determinate ? { width: `${percent ?? 0}%` } : undefined}
      />
    </div>
  );
}
