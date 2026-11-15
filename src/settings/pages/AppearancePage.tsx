import type { Accent, Appearance, Language, NumberFormat } from "../../api";
import { useT } from "../../i18n";
import { useSettings, useUpdateSettings } from "../../queries/settings";
import { Group, Row, Segmented, Switch } from "../controls";

const ACCENTS: Accent[] = ["system", "orange", "gold", "red", "magenta", "purple", "indigo", "blue", "teal", "graphite"];
const capitalize = (s: string) => s[0].toUpperCase() + s.slice(1);

export function AppearancePage() {
  const { data: settings } = useSettings();
  const { mutate } = useUpdateSettings();
  const text = useT().appearance;
  return (
    <>
      <Group title={text.languageGroup} anchor="language-group">
        <Row label={text.language} description={text.languageDescription} anchor="language">
          <Segmented<Language>
            label={text.language}
            options={[
              { value: "en", label: "English" },
              { value: "de", label: "Deutsch" },
            ]}
            value={settings?.language}
            onChange={(language) => mutate({ language })}
          />
        </Row>
      </Group>
      <Group title="Theme" anchor="theme">
        <Row label="Mode" anchor="mode">
          <Segmented<Appearance>
            label="Mode"
            options={[
              { value: "system", label: "Automatic" },
              { value: "light", label: "Light" },
              { value: "dark", label: "Dark" },
            ]}
            value={settings?.appearance}
            onChange={(appearance) => mutate({ appearance })}
          />
        </Row>
        <Row label="Accent" description="Used for selection, links and the main button." anchor="accent">
          <div className="accent-dots" role="radiogroup" aria-label="Accent">
            {ACCENTS.map((accent) => (
              <button
                key={accent}
                type="button"
                role="radio"
                aria-checked={settings?.accent === accent}
                aria-label={capitalize(accent)}
                title={capitalize(accent)}
                className={`accent-dot accent-dot--${accent}`}
                onClick={() => mutate({ accent })}
              />
            ))}
          </div>
        </Row>
      </Group>
      <Group title="Amounts" anchor="amounts">
        <Row label="Number format" anchor="number-format">
          <Segmented<NumberFormat>
            label="Number format"
            options={[
              { value: "dot_decimal", label: "1,234.56" },
              { value: "comma_decimal", label: "1.234,56" },
            ]}
            value={settings?.numberFormat}
            onChange={(numberFormat) => mutate({ numberFormat })}
          />
        </Row>
        <Switch
          label="Show foreign currency next to amounts"
          description="For example USD 24.00 beside the euro amount."
          anchor="original-currency"
          checked={settings?.showOriginalCurrency ?? true}
          onChange={(showOriginalCurrency) => mutate({ showOriginalCurrency })}
        />
      </Group>
    </>
  );
}
