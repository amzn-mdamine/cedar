/*
 * Copyright 2022-2023 Amazon.com, Inc. or its affiliates. All Rights Reserved.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use cedar_policy_core::{ast::PolicyID, parser::SourceInfo};
use thiserror::Error;

use crate::TypeErrorKind;

/// Contains the result of policy validation. The result includes the list of of
/// issues found by the validation and whether validation succeeds or fails.
/// Validation succeeds if there are no fatal errors.  There are currently no
/// non-fatal warnings, so any issues found will cause validation to fail.
#[derive(Debug)]
pub struct ValidationResult<'a> {
    validation_errors: Vec<ValidationError<'a>>,
}

impl<'a> ValidationResult<'a> {
    pub(crate) fn new(validation_errors: impl Iterator<Item = ValidationError<'a>>) -> Self {
        Self {
            validation_errors: validation_errors.collect::<Vec<_>>(),
        }
    }

    /// True when validation passes. There are no fatal errors.
    pub fn validation_passed(&self) -> bool {
        self.validation_errors.is_empty()
    }

    /// Get the list of errors found by the validator.
    pub fn validation_errors(&self) -> impl Iterator<Item = &ValidationError> {
        self.validation_errors.iter()
    }

    /// Get the list of errors found by the validator.
    pub fn into_validation_errors(self) -> impl Iterator<Item = ValidationError<'a>> {
        self.validation_errors.into_iter()
    }
}

/// An error generated by the validator when it finds a potential problem in a
/// policy. The error contains a enumeration that specifies the kind of problem,
/// and provides details specific to that kind of problem. The error also records
/// where the problem was encountered.
#[derive(Debug)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct ValidationError<'a> {
    location: SourceLocation<'a>,
    error_kind: ValidationErrorKind,
}

impl<'a> ValidationError<'a> {
    pub(crate) fn with_policy_id(
        id: &'a PolicyID,
        source_info: Option<SourceInfo>,
        error_kind: ValidationErrorKind,
    ) -> Self {
        Self {
            error_kind,
            location: SourceLocation::new(id, source_info),
        }
    }

    /// Deconstruct this into its component source location and error kind.
    pub fn into_location_and_error_kind(self) -> (SourceLocation<'a>, ValidationErrorKind) {
        (self.location, self.error_kind)
    }

    /// Extract details about the exact issue detected by the validator.
    pub fn error_kind(&self) -> &ValidationErrorKind {
        &self.error_kind
    }

    /// Extract the location where the validator found the issue.
    pub fn location(&self) -> &SourceLocation {
        &self.location
    }
}

/// Represents a location in Cedar policy source.
#[derive(Debug, Eq, PartialEq)]
pub struct SourceLocation<'a> {
    policy_id: &'a PolicyID,
    source_info: Option<SourceInfo>,
}

impl<'a> SourceLocation<'a> {
    fn new(policy_id: &'a PolicyID, source_info: Option<SourceInfo>) -> Self {
        Self {
            policy_id,
            source_info,
        }
    }

    /// Get the `PolicyId` for the policy at this source location.
    pub fn policy_id(&self) -> &'a PolicyID {
        self.policy_id
    }

    pub fn source_info(&self) -> &Option<SourceInfo> {
        &self.source_info
    }

    pub fn into_source_info(self) -> Option<SourceInfo> {
        self.source_info
    }
}

/// Enumeration of the possible diagnostic error that could be found by the
/// verification steps.
#[derive(Debug, Error)]
#[cfg_attr(test, derive(Eq, PartialEq))]
#[non_exhaustive]
pub enum ValidationErrorKind {
    /// An entity type was seen in a policy but was not found in the schema.
    #[error(
        "Unrecognized entity type {}{}",
        .0.actual_entity_type,
        match &.0.suggested_entity_type {
            Some(s) => format!(", did you mean {}?", s),
            None => "".to_string()
        }
    )]
    UnrecognizedEntityType(UnrecognizedEntityType),
    /// An action id was seen in a policy but was not found in the schema.
    #[error(
        "Unrecognized action id {}{}",
        .0.actual_action_id,
        match &.0.suggested_action_id {
            Some(s) => format!(", did you mean {}?", s),
            None => "".to_string()
        }
    )]
    UnrecognizedActionId(UnrecognizedActionId),
    /// There is no action satisfying the action head constraint that can be
    /// applied to a principal and resources that both satisfy their respective
    /// head conditions.
    #[error(
        "Unable to find an applicable action given the policy head constraints{}{}",
        if .0.would_in_fix_principal { ". Note: Try replacing `==` with `in` in the principal clause" } else { "" },
        if .0.would_in_fix_resource { ". Note: Try replacing `==` with `in` in the resource clause" } else { "" }
    )]
    InvalidActionApplication(InvalidActionApplication),
    /// A type error was found by the type checker.
    #[error(transparent)]
    TypeError(TypeErrorKind),
    /// An unspecified entity was used in a policy. This should be impossible,
    /// assuming that the policy was constructed using the parser.
    #[error(
        "Unspecified entity with eid {}. Unspecified entities cannot be used in policies",
        .0.entity_id,
    )]
    UnspecifiedEntity(UnspecifiedEntity),
}

impl ValidationErrorKind {
    pub(crate) fn unrecognized_entity_type(
        actual_entity_type: String,
        suggested_entity_type: Option<String>,
    ) -> ValidationErrorKind {
        Self::UnrecognizedEntityType(UnrecognizedEntityType {
            actual_entity_type,
            suggested_entity_type,
        })
    }

    pub(crate) fn unrecognized_action_id(
        actual_action_id: String,
        suggested_action_id: Option<String>,
    ) -> ValidationErrorKind {
        Self::UnrecognizedActionId(UnrecognizedActionId {
            actual_action_id,
            suggested_action_id,
        })
    }

    pub(crate) fn invalid_action_application(
        would_in_fix_principal: bool,
        would_in_fix_resource: bool,
    ) -> ValidationErrorKind {
        Self::InvalidActionApplication(InvalidActionApplication {
            would_in_fix_principal,
            would_in_fix_resource,
        })
    }

    pub(crate) fn type_error(type_error: TypeErrorKind) -> ValidationErrorKind {
        Self::TypeError(type_error)
    }

    pub(crate) fn unspecified_entity(entity_id: String) -> ValidationErrorKind {
        Self::UnspecifiedEntity(UnspecifiedEntity { entity_id })
    }
}

/// Structure containing details about an unrecognized entity type error.
#[derive(Debug)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct UnrecognizedEntityType {
    /// The entity type seen in the policy.
    pub(crate) actual_entity_type: String,
    /// An entity type from the schema that the user might reasonably have
    /// intended to write.
    pub(crate) suggested_entity_type: Option<String>,
}

/// Structure containing details about an unrecognized action id error.
#[derive(Debug)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct UnrecognizedActionId {
    /// Action Id seen in the policy.
    pub(crate) actual_action_id: String,
    /// An action id from the schema that the user might reasonably have
    /// intended to write.
    pub(crate) suggested_action_id: Option<String>,
}

/// Structure containing details about an invalid action application error.
#[derive(Debug)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct InvalidActionApplication {
    pub(crate) would_in_fix_principal: bool,
    pub(crate) would_in_fix_resource: bool,
}

/// Structure containing details about an unspecified entity error.
#[derive(Debug)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct UnspecifiedEntity {
    /// EID of the unspecified entity.
    pub(crate) entity_id: String,
}
