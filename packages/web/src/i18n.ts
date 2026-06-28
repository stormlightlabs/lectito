import type { I18n } from "@lingui/core";

export const defaultLocale = "en";

export async function loadCatalog(locale: string, i18n: I18n) {
  const catalog = await import(`./locales/${locale}.po`);
  i18n.loadAndActivate({ locale, messages: catalog.messages });
}
