import { useQuery } from "@tanstack/react-query";
import { api, type SubscriptionForm } from "../api";

export const SUBSCRIPTIONS_KEY = ["subscriptions"] as const;
export const SUBSCRIPTION_KEY = ["subscription"] as const;
export const MATCH_COUNT_KEY = ["subscription-match-count"] as const;

export const useSubscriptions = (query: string) =>
  useQuery({ queryKey: [...SUBSCRIPTIONS_KEY, query], queryFn: () => api.subscriptionsView(query), placeholderData: (previous) => previous });

export const useSubscription = (id: string | null) =>
  useQuery({ queryKey: [...SUBSCRIPTION_KEY, id], queryFn: () => api.subscriptionDetail(id ?? ""), enabled: id !== null, meta: { quiet: true } });

/** How many lines the sheet's bank text and amount match, kept while the next count loads. */
export const useMatchCount = (form: SubscriptionForm) =>
  useQuery({ queryKey: [...MATCH_COUNT_KEY, form], queryFn: () => api.subscriptionMatchCount(form), placeholderData: (previous) => previous });
