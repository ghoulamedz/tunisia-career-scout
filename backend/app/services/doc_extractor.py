"""
Document requirements extraction service.
Parses PDF announcements and extracts required documents (diplomas, attestations, etc.).
"""

import re
import logging
from typing import Any

import httpx

from .summary import generate_summary
from ..core.config import get_settings

logger = logging.getLogger(__name__)

# Common document types in Tunisian job/concours announcements
DOCUMENT_PATTERNS = [
    # Diploma patterns
    (r"(dipl[oô]me|licence|master|bachelor|ing[ée]nieur|BAC|baccalaur[ée]|\bbac\b|\blicence\b|\bmaster\b|\bing[\u00e9e]nieur\b)", "diploma"),
    (r"(attestation|attestat[ée]|certificat)\s+(de|d['’])?\s*(\w+)", "attestation"),
    (r"(carte d'\w+|carte nationale|carte d'identit[ée])", "identity_card"),
    (r"(casier judiciaire|casier n[\u00b0b]?)\s*\d*", "criminal_record"),
    (r"(photo d'?identit[ée]|photo d'identit[ée])", "photo"),
    (r"(lettre de motivation|lettre de recommandation|recommendation)", "motivation_letter"),
    (r"(cv|curriculum vitae|r[\u00e9e]sum[\u00e9e])", "cv"),
    (r"(relev[ée] de notes|relev['’] de notes)", "transcript"),
    (r"(certificat de travail|certificat de salari[ée]|attestation de travail)", "work_certificate"),
    (r"(certificat de participation|certificat de formation)", "training_certificate"),
    (r"(concours|examen|test)", "exam_result"),
    (r"(permis de conduire|permis)", "driving_license"),
    (r"(assurance|mutuelle)", "insurance"),
    (r"(taux de change|dipl[oô]me ?de ?francisation|dipl['’]me)", "language_certificate"),
    (r"(taux d'int[eé]gration|taux)", "integration_certificate"),
]


def extract_documents_from_text(text: str) -> list[dict]:
    """Extract required documents from text using pattern matching."""
    documents = []
    found_types = set()

    for pattern, doc_type in DOCUMENT_PATTERNS:
        matches = re.finditer(pattern, text, re.IGNORECASE)
        for match in matches:
            doc_text = match.group(0).strip()
            if doc_type not in found_types:
                documents.append({
                    "type": doc_type,
                    "name": doc_text[:200],
                    "required": True,
                })
                found_types.add(doc_type)

    return documents


async def extract_documents_from_posting(
    posting_data: dict,
    posting_type: str = "job",
) -> dict:
    """
    Extract required documents from a posting.
    Uses LLM for quality extraction + rule-based fallback.

    Returns:
        {documents, raw_text, extraction_method}
    """
    description = posting_data.get("description", "")
    raw = posting_data.get("raw", {})
    details = raw.get("details", raw.get("required_docs", {}))

    # Try LLM-based extraction first
    try:
        llm_result = await generate_summary(
            posting_data,
            {"skills": [], "education": [], "experience": []},
            posting_type=posting_type,
            include_documents=True,
        )
        if llm_result.get("required_documents"):
            return {
                "documents": llm_result["required_documents"],
                "raw_text": llm_result.get("summary", ""),
                "extraction_method": "llm",
            }
    except Exception as e:
        logger.warning(f"LLM document extraction failed: {e}")

    # Rule-based fallback
    text_to_search = description
    if isinstance(details, dict):
        text_to_search += " " + " ".join(str(v) for v in details.values() if v)

    docs = extract_documents_from_text(text_to_search)

    return {
        "documents": docs,
        "raw_text": text_to_search[:500],
        "extraction_method": "rules",
    }


async def extract_docs_from_pdf_announcement(pdf_url: str) -> dict:
    """
    Fetch a PDF announcement and extract required documents.
    Downloads the PDF, extracts text, then applies document extraction.
    """
    try:
        async with httpx.AsyncClient(timeout=30, verify=False) as client:
            response = await client.get(pdf_url)
            response.raise_for_status()

        # Save temp PDF
        import tempfile
        import os
        with tempfile.NamedTemporaryFile(delete=False, suffix=".pdf") as f:
            f.write(response.content)
            tmp_path = f.name

        try:
            from .resume_parser import parse_resume
            result = parse_resume(tmp_path)
            docs = extract_documents_from_text(result.get("raw_text", ""))
            return {
                "documents": docs,
                "raw_text": result.get("raw_text", "")[:1000],
                "extraction_method": "pdf_rules",
            }
        finally:
            os.unlink(tmp_path)

    except Exception as e:
        logger.error(f"PDF document extraction failed: {e}")
        return {
            "documents": [],
            "raw_text": "",
            "extraction_method": "failed",
            "error": str(e),
        }