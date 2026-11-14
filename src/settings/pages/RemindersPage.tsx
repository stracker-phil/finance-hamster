import type { Reminder } from "../../api";
import { useSettings, useUpdateSettings } from "../../queries/settings";
import { Group, Row, Segmented, Switch } from "../controls";

const REMINDERS: { value: Reminder; label: string }[] = [
  { value: "off", label: "Off" },
  { value: "one_day", label: "1 day" },
  { value: "three_days", label: "3 days" },
  { value: "one_week", label: "1 week" },
];

export function RemindersPage() {
  const { data: settings } = useSettings();
  const { mutate } = useUpdateSettings();
  return (
    <>
      <Group title="Subscriptions" anchor="reminders-subscriptions">
        <Row label="Default reminder for new subscriptions" description="Each subscription keeps its own; change it in its sheet." anchor="default-reminder">
          <Segmented<Reminder> label="Default reminder" options={REMINDERS} value={settings?.defaultReminder} onChange={(defaultReminder) => mutate({ defaultReminder })} />
        </Row>
        <Switch
          label="Tell me when a price changes"
          description="When a charge differs from the usual amount."
          anchor="price-changes"
          checked={settings?.remindPriceChanges ?? true}
          onChange={(remindPriceChanges) => mutate({ remindPriceChanges })}
        />
      </Group>
      <Group title="Statements" anchor="reminders-statements">
        <Row label="Remind me to import after" description="Counted from the end of the last statement of each account." htmlFor="import-after" anchor="import-after">
          <input
            id="import-after"
            className="field settings-number"
            type="number"
            min={1}
            value={settings?.importAfterDays ?? 30}
            onChange={(e) => {
              const days = Number(e.target.value);
              if (days >= 1) mutate({ importAfterDays: days });
            }}
          />
          <span className="settings-value">days</span>
        </Row>
        <Switch
          label="Tell me about gaps"
          description="When a statement doesn’t continue the previous one."
          anchor="gaps"
          checked={settings?.remindGaps ?? true}
          onChange={(remindGaps) => mutate({ remindGaps })}
        />
      </Group>
      <Group title="Tax year" anchor="reminders-tax-year">
        <Switch
          label="Filing reminder"
          description="On 1 March, when the previous tax year can be closed."
          anchor="filing"
          checked={settings?.remindFiling ?? true}
          onChange={(remindFiling) => mutate({ remindFiling })}
        />
        <Switch
          label="Year check"
          description="From 1 November until it ran, and again after 31 Dec until it ran once more."
          anchor="year-check"
          checked={settings?.remindQuestions ?? true}
          onChange={(remindQuestions) => mutate({ remindQuestions })}
        />
      </Group>
    </>
  );
}
