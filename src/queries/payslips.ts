import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

export const PAYSLIPS_KEY = ["payslips"] as const;
export const PAYSLIP_KEY = ["payslip"] as const;

export const usePayslipsYear = (year: number, enabled = true) => useQuery({ queryKey: [...PAYSLIPS_KEY, year], queryFn: () => api.payslipsYear(year), enabled });

export const usePayslip = (id: string, enabled = true) => useQuery({ queryKey: [...PAYSLIP_KEY, id], queryFn: () => api.payslipView(id), enabled, meta: { quiet: true } });
