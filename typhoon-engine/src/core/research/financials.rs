use serde::{Deserialize, Serialize};

/// FA — one fiscal period of an Income Statement.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IncomeStatement {
    pub date: String,   // period end YYYY-MM-DD
    pub period: String, // "FY" | "Q1" | "Q2" | "Q3" | "Q4"
    pub revenue: f64,
    pub cost_of_revenue: f64,
    pub gross_profit: f64,
    pub research_and_development: f64,
    pub selling_general_admin: f64,
    pub operating_expenses: f64,
    pub operating_income: f64,
    pub interest_expense: f64,
    pub ebitda: f64,
    pub income_before_tax: f64,
    pub income_tax_expense: f64,
    pub net_income: f64,
    pub eps: f64,
    pub eps_diluted: f64,
    pub weighted_shares_out: f64,
}

/// FA — one fiscal period of a Balance Sheet.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BalanceSheet {
    pub date: String,
    pub period: String,
    pub cash_and_equiv: f64,
    pub short_term_investments: f64,
    pub net_receivables: f64,
    pub inventory: f64,
    pub total_current_assets: f64,
    pub property_plant_equipment: f64,
    pub goodwill: f64,
    pub intangible_assets: f64,
    pub long_term_investments: f64,
    pub total_non_current_assets: f64,
    pub total_assets: f64,
    pub accounts_payable: f64,
    pub short_term_debt: f64,
    pub total_current_liabilities: f64,
    pub long_term_debt: f64,
    pub total_non_current_liabilities: f64,
    pub total_liabilities: f64,
    pub common_stock: f64,
    pub retained_earnings: f64,
    pub total_equity: f64,
    pub total_debt: f64,
    pub net_debt: f64,
}

/// FA — one fiscal period of a Cash Flow Statement.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CashFlowStatement {
    pub date: String,
    pub period: String,
    pub net_income: f64,
    pub depreciation_amortization: f64,
    pub stock_based_comp: f64,
    pub change_working_capital: f64,
    pub cash_from_operations: f64,
    pub capex: f64,
    pub acquisitions: f64,
    pub investments_purchases: f64,
    pub cash_from_investing: f64,
    pub debt_repayment: f64,
    pub dividends_paid: f64,
    pub stock_repurchases: f64,
    pub cash_from_financing: f64,
    pub net_change_cash: f64,
    pub free_cash_flow: f64,
}

/// FA — combined bundle of all 3 statements × (annual/quarterly) for a symbol.
/// Serialized as a single JSON blob in research_financials so one SQL row covers the whole view.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FinancialStatements {
    pub income_annual: Vec<IncomeStatement>,
    pub income_quarterly: Vec<IncomeStatement>,
    pub balance_annual: Vec<BalanceSheet>,
    pub balance_quarterly: Vec<BalanceSheet>,
    pub cashflow_annual: Vec<CashFlowStatement>,
    pub cashflow_quarterly: Vec<CashFlowStatement>,
}
