import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

export const REVIEW_KEY = ["review-queue"] as const;

export const useReviewQueue = () => useQuery({ queryKey: REVIEW_KEY, queryFn: api.reviewQueue });
