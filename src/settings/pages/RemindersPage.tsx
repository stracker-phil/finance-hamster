import type { Reminder } from "../../api";
import { useT } from "../../i18n";
import { useSettings, useUpdateSettings } from "../../queries/settings";
import { Group, Row, Segmented, Switch } from "../controls";

export function RemindersPage() {
  const { data: settings } = useSettings();
  const { mutate } = useUpdateSettings();
  const text = useT().settings.reminders;
  const reminders: { value: Reminder; label: string }[] = [
    { value: "off", label: text.off },
    { value: "one_day", label: text.oneDay },
    { value: "three_days", label: text.threeDays },
    { value: "one_week", label: text.oneWeek },
  ];
  return (
    <>
      <Group title={text.subscriptions} anchor="reminders-subscriptions">
        <Row label={text.defaultReminder} description={text.defaultReminderDescription} anchor="default-reminder">
          <Segmented<Reminder> label={text.defaultReminderLabel} options={reminders} value={settings?.defaultReminder} onChange={(defaultReminder) => mutate({ defaultReminder })} />
        </Row>
        <Switch
          label={text.priceChanges}
          description={text.priceChangesDescription}
          anchor="price-changes"
          checked={settings?.remindPriceChanges ?? true}
          onChange={(remindPriceChanges) => mutate({ remindPriceChanges })}
        />
      </Group>
      <Group title={text.statements} anchor="reminders-statements">
        <Row label={text.importAfter} description={text.importAfterDescription} htmlFor="import-after" anchor="import-after">
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
          <span className="settings-value">{text.days}</span>
        </Row>
        <Switch
          label={text.gaps}
          description={text.gapsDescription}
          anchor="gaps"
          checked={settings?.remindGaps ?? true}
          onChange={(remindGaps) => mutate({ remindGaps })}
        />
      </Group>
      <Group title={text.taxYear} anchor="reminders-tax-year">
        <Switch
          label={text.filing}
          description={text.filingDescription}
          anchor="filing"
          checked={settings?.remindFiling ?? true}
          onChange={(remindFiling) => mutate({ remindFiling })}
        />
        <Switch
          label={text.yearEnd}
          description={text.yearEndDescription}
          anchor="year-end"
          checked={settings?.remindQuestions ?? true}
          onChange={(remindQuestions) => mutate({ remindQuestions })}
        />
      </Group>
    </>
  );
}
