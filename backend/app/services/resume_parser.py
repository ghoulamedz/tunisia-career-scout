"""
Resume parsing service — extracts skills, education, experience from CV.
Supports .txt, .pdf (via basic extraction), and structured formats.
"""

import os
import re
import uuid
from pathlib import Path
from typing import Any

# Optional: pdfplumber / pypdf for PDF parsing
try:
    import pypdf
    _PDF_AVAILABLE = True
except ImportError:
    _PDF_AVAILABLE = False

try:
    import pdfplumber
    _PDFPLUMBER_AVAILABLE = True
except ImportError:
    _PDFPLUMBER_AVAILABLE = False

# Common Tunisian job keywords for skill detection
TUNISIAN_SKILLS = [
    "gestion de projet", "gestion de portefeuille", "analyse de donnees",
    "intelligence artificielle", "machine learning", "deep learning",
    "python", "java", "javascript", "typescript", "react", "node.js",
    "django", "flask", "sql", "postgresql", "mysql", "mongodb",
    "cloud", "aws", "azure", "gcp", "devops", "ci/cd", "docker", "kubernetes",
    "digitaux", "marketing digital", "rse", "responsabilite sociale",
    "commerçal", "commercial", "relation client", "service client",
    "accompagnement", "coach", "formateur", "pedagogie",
    "droit", "juridique", "contrats", "rédaction", "auditeur",
    "comptabilité", "finance", "budget", "audit", "controle",
    "ressources humaines", "rh", "recrutement", "paie", "gratifications",
    "ingénierie", "ingénieur", "bâtiment", "architecture", "électricité",
    "maintenance", "qualité", "sécurité", "environnement",
    "santé", "médecine", "pharmacie", "infirmier",
    "éducation", "enseignement", "professionnel", "indemnités",
    "administratif", "secrétariat", "logistique", "supply chain",
    "convoyage", "voyage", "tourisme", "hôtellerie",
    "télécoms", "réseaux", "sécurité informatique", "cyber",
    "production", "qualité", "industrial", "mécanique",
    "électronique", "énergie", "renouvelable", "solaire", "éolien",
]

DEFAULT_SKILLS = [
    "gestion de projet", "communication", "travail d'équipe",
    "anglais", "français", "arabe", "adaptabilité",
    "résolution de problèmes", "organisation", "motivation",
    "autonomie", "esprit d'équipe", "veille",
]


def extract_pdf_text(path: str) -> str:
    """Extract text from a PDF file. Returns empty string on failure."""
    text = ""
    if _PDFPLUMBER_AVAILABLE:
        try:
            with pdfplumber.open(path) as pdf:
                for page in pdf.pages:
                    t = page.extract_text()
                    if t:
                        text += t + "\n"
            return text
        except Exception:
            pass

    if _PDF_AVAILABLE:
        try:
            reader = pypdf.PdfReader(path)
            for page in reader.pages:
                t = page.extract_text()
                if t:
                    text += t + "\n"
            return text
        except Exception:
            pass

    # Fallback: try to read as plain text (sometimes PDFs are text-based)
    try:
        with open(path, "r", encoding="utf-8", errors="ignore") as f:
            return f.read()
    except Exception:
        return ""


def extract_text_from_file(file_path: str) -> str:
    """Extract text from a file based on extension."""
    ext = os.path.splitext(file_path)[1].lower()
    if ext == ".pdf":
        return extract_pdf_text(file_path)
    elif ext in (".txt", ".md"):
        try:
            with open(file_path, "r", encoding="utf-8", errors="ignore") as f:
                return f.read()
        except Exception:
            return ""
    else:
        # Try to read as plain text
        try:
            with open(file_path, "r", encoding="utf-8", errors="ignore") as f:
                return f.read()
        except Exception:
            return ""


def normalize_skills(text: str) -> list[str]:
    """Extract and normalize skill keywords from text."""
    skills = set()
    text_lower = text.lower()

    for skill in TUNISIAN_SKILLS:
        if skill.lower() in text_lower:
            skills.add(skill)

    # Also find standalone effectif terms
    effectif_terms = [
        "développement", "développeuse", "ingénieur", "commercial",
        "gestionnaire", "responsable", "chef", "directeur", "chef de projet",
        "analyste", "architecte", "conseiller", "assistant(e)",
    ]
    for term in effectif_terms:
        if term in text_lower:
            skills.add(term)

    return list(skills)


def extract_education(text: str) -> list[str]:
    """Extract education level indicators."""
    education_levels = []
    text_lower = text.lower()

    if any(kw in text_lower for kw in ["bac", "baccalauréat", "bac +2", "bt", "bts", "taf"]):
        education_levels.append("Baccalauréat / BT / BTS")
    if any(kw in text_lower for kw in ["licence", "lic", "bachelor", "licence pro"]):
        education_levels.append("Licence (Bac+3)")
    if any(kw in text_lower for kw in ["master", "mastère", "masters"]):
        education_levels.append("Master (Bac+5)")
    if any(kw in text_lower for kw in ["doctorat", "phd", "dpt"]):
        education_levels.append("Doctorat")
    if any(kw in text_lower for kw in ["master +", "master2", "master 2"]):
        education_levels.append("Master 2")
    if any(kw in text_lower for kw in ["master +2", "master + 2"]):
        education_levels.append("Master +2")
    if any(kw in text_lower for kw in ["licence +", "licence 2"]):
        education_levels.append("Licence 2 / Master courant")

    return education_levels


def extract_experience(text: str) -> list[str]:
    """Extract experience indicators."""
    experience = []
    text_lower = text.lower()

    experience_keywords = [
        "année d'expérience", "année d'expérience", "ans d'expérience",
        "expérience professionnelle", "expérience professionnelle",
        "expérience dans", "expérience en",
        "responsabilité", "chef de projet", "chef d'équipe",
        "gestion de dossier", "gestion de dossiers", "supervision",
        "suivi", "gestion des gens", "recrutement", "entretien",
    ]
    for kw in experience_keywords:
        if kw in text_lower:
            experience.append(kw)

    return experience


def parse_resume(file_path: str) -> dict[str, Any]:
    """
    Parse a resume file and return structured data.
    Returns: {skills, education, experience, raw_text}
    """
    text = extract_text_from_file(file_path)

    return {
        "raw_text": text,
        "skills": normalize_skills(text),
        "education": extract_education(text),
        "experience": extract_experience(text),
    }
